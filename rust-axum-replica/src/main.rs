mod app;
mod broadcaster;
mod config;
mod logging;
mod routes;
mod services;
mod state;

use axum::body::Body;
use axum::http::StatusCode;
use tower::util::ServiceExt;
use tracing::info;

use crate::app::build_app;
use crate::logging::init_logging;

#[tokio::test]
async fn health_route_works() {
    let app = build_app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn crawl_route_requires_api_key() {
    let app = build_app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/crawl/nettruyen/comic")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"slug-and-id":"one-piece-1"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn crawl_route_accepts_valid_api_key() {
    let previous = std::env::var("CRAWL_API_KEY").ok();
    unsafe {
        std::env::set_var("CRAWL_API_KEY", "test-key");
    }

    let app = build_app();
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri("/api/v1/crawl/nettruyen/comic")
                .header("content-type", "application/json")
                .header("x-crawl-api-key", "test-key")
                .body(Body::from(r#"{"slug-and-id":"   "}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    if let Some(value) = previous {
        unsafe {
            std::env::set_var("CRAWL_API_KEY", value);
        }
    } else {
        unsafe {
            std::env::remove_var("CRAWL_API_KEY");
        }
    }

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::main]
async fn main() {
    init_logging();
    let app = build_app();
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    info!("server.starting port={}", 3000);
    axum::serve(listener, app).await.unwrap();
}
