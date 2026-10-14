use std::net::SocketAddr;
use tracing::info;

use comic_crawler::app::{build_app_with_state, spawn_crawl_queue_worker};
use comic_crawler::config::preflight_checks;
use comic_crawler::logging::init_logging;
use comic_crawler::state::AppState;

mod grpc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "\n\
         ========================================\n\
                    WARADA CRAWL API\n\
               Rust + Axum | :3000\n\
         ========================================\n"
    );

    dotenvy::dotenv().ok();

    init_logging();

    if let Err(err) = preflight_checks().await {
        eprintln!("Preflight validation failed: {err}");
        return Err(err.into());
    }

    let state = AppState::default();
    spawn_crawl_queue_worker(state.clone());
    let app = build_app_with_state(state.clone());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    let grpc_address: SocketAddr = std::env::var("GRPC_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".to_string())
        .parse()?;
    info!(
        "server.starting http_port={} grpc_address={}",
        3000, grpc_address
    );

    let grpc_server = tonic::transport::Server::builder()
        .add_service(crawler_proto::crawler_server::CrawlerServer::new(
            grpc::CrawlerGrpc::new(state),
        ))
        .serve(grpc_address);
    tokio::select! {
        result = grpc_server => result?,
        result = axum::serve(listener, app) => result?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use comic_crawler::state::{ChapterRecord, ComicRecord};
    use crawler_proto::{ComicRequest, Empty, crawler_server::Crawler};

    #[tokio::test]
    async fn grpc_catalog_reads_memory_state_and_returns_expected_errors() {
        let previous_database_path = std::env::var("DATABASE_PATH").ok();
        unsafe {
            std::env::remove_var("DATABASE_PATH");
        }
        let state = AppState::default();
        let service = grpc::CrawlerGrpc::new(state.clone());

        assert!(
            service
                .list_comics(tonic::Request::new(Empty {}))
                .await
                .unwrap()
                .into_inner()
                .comics
                .is_empty()
        );
        assert_eq!(
            service
                .get_comic(tonic::Request::new(ComicRequest {
                    slug: "missing".to_owned(),
                }))
                .await
                .unwrap_err()
                .code(),
            tonic::Code::NotFound
        );
        assert_eq!(
            service
                .list_chapters(tonic::Request::new(ComicRequest {
                    slug: "missing".to_owned(),
                }))
                .await
                .unwrap_err()
                .code(),
            tonic::Code::NotFound
        );

        state
            .insert_comic(ComicRecord {
                id: 1,
                slug: "one-piece".to_owned(),
                title: "One Piece".to_owned(),
                author: Some("Eiichiro Oda".to_owned()),
                description: None,
                status: "OnGoing".to_owned(),
                chapter_count: Some(1),
                crawling_status: "0000".to_owned(),
                origin_id: Some("123".to_owned()),
                origin_url: Some("https://example.test/one-piece".to_owned()),
                origin_path_params: Some("one-piece-123".to_owned()),
                thumb_image_id: None,
            })
            .await;
        state
            .insert_chapter(ChapterRecord {
                id: 2,
                comic_id: 1,
                chapter_num: "1".to_owned(),
                position: 1,
                crawling_status: "0000".to_owned(),
                origin_url: "https://example.test/chapter/1".to_owned(),
                origin_path_params: "one-piece/1".to_owned(),
            })
            .await;

        let comic = service
            .get_comic(tonic::Request::new(ComicRequest {
                slug: "one-piece".to_owned(),
            }))
            .await
            .unwrap()
            .into_inner()
            .comic
            .unwrap();
        assert_eq!(comic.id, 1);
        assert_eq!(comic.author.as_deref(), Some("Eiichiro Oda"));
        let chapters = service
            .list_chapters(tonic::Request::new(ComicRequest {
                slug: "one-piece".to_owned(),
            }))
            .await
            .unwrap()
            .into_inner()
            .chapters;
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].chapter_number, "1");
        assert_eq!(
            service
                .list_comics(tonic::Request::new(Empty {}))
                .await
                .unwrap()
                .into_inner()
                .comics
                .len(),
            1
        );
        assert_eq!(
            service
                .list_crawl_jobs(tonic::Request::new(Empty {}))
                .await
                .unwrap_err()
                .code(),
            tonic::Code::Unavailable
        );

        unsafe {
            if let Some(path) = previous_database_path {
                std::env::set_var("DATABASE_PATH", path);
            } else {
                std::env::remove_var("DATABASE_PATH");
            }
        }
    }
}
