use tracing::info;

use axum_replica::app::build_app;
use axum_replica::config::preflight_checks;
use axum_replica::logging::init_logging;

#[tokio::main]
async fn main() {
    init_logging();

    if let Err(err) = preflight_checks().await {
        eprintln!("Preflight validation failed: {err}");
        std::process::exit(1);
    }

    let app = build_app();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!(
        "\n\
         ========================================\n\
                    WARADA CRAWL API\n\
               Rust + Axum | :3000\n\
         ========================================\n"
    );
    info!("server.starting port={}", 3000);
    axum::serve(listener, app).await.unwrap();
}
