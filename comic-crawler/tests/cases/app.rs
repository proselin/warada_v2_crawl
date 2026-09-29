use axum::body::Body;
use axum::http::header::CONTENT_TYPE;
use axum::http::{Request, StatusCode};
use axum_replica::app::build_app;
use axum_replica::config::{nettruyen_url, preflight_checks};
use tower::util::ServiceExt;

use super::common::{ENV_LOCK, restore_env};

#[tokio::test]
async fn app_routes_work_and_config_behaves_as_expected() {
    let _env_lock = ENV_LOCK.lock().unwrap();
    let previous_key = std::env::var("CRAWL_API_KEY").ok();
    let previous_url = std::env::var("NETTRUYEN_URL").ok();
    let previous_db = std::env::var("DATABASE_URL").ok();
    unsafe {
        std::env::remove_var("NETTRUYEN_URL");
    }
    assert_eq!(nettruyen_url(), "https://nettruyenar.com");

    unsafe {
        std::env::set_var("NETTRUYEN_URL", "https://example.test");
    }
    assert_eq!(nettruyen_url(), "https://example.test");

    unsafe {
        std::env::set_var("DATABASE_URL", "");
        std::env::remove_var("CRAWL_API_KEY");
        std::env::remove_var("NETTRUYEN_URL");
    }
    assert!(preflight_checks().await.is_err());
    restore_env("DATABASE_URL", previous_db.clone());
    restore_env("NETTRUYEN_URL", previous_url.clone());
    restore_env("CRAWL_API_KEY", previous_key.clone());

    let response = build_app()
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let response = build_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/crawl/nettruyen/comic")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"slug-and-id":"one-piece-1"}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

    unsafe {
        std::env::set_var("CRAWL_API_KEY", "test-key");
    }
    let response = build_app()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/crawl/nettruyen/comic")
                .header(CONTENT_TYPE, "application/json")
                .header("x-crawl-api-key", "test-key")
                .body(Body::from(r#"{"slug-and-id":"   "}"#))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    restore_env("CRAWL_API_KEY", previous_key);
    restore_env("NETTRUYEN_URL", previous_url);
    restore_env("DATABASE_URL", previous_db);
}
