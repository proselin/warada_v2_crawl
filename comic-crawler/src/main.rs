use tracing::info;

use comic_crawler::app::build_app;
use comic_crawler::config::preflight_checks;
use comic_crawler::logging::init_logging;

#[tokio::main]
async fn main() {

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
        std::process::exit(1);
    }

    let app = build_app();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    info!("server.starting port={}", 3000);
    axum::serve(listener, app).await.unwrap();
}
