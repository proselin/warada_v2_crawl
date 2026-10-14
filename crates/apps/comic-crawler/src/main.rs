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
