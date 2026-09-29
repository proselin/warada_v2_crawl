use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;

use crate::routes::crawl::{crawl_nettruyen_comic, progress_stream, retry_failed_chapters};
use crate::state::AppState;

pub fn build_app() -> Router {
    let state = AppState::default();
    let crawl_router = Router::new()
        .route("/api/v1/crawl/nettruyen/comic", post(crawl_nettruyen_comic))
        .route("/api/v1/crawl/nettruyen/comic/{slug}/retry", post(retry_failed_chapters))
        .route("/api/v1/crawl/progress/{comic_slug}", get(progress_stream))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_crawl_api_key))
        .with_state(state.clone());

    Router::new()
        .route("/health", get(health))
        .merge(crawl_router)
        .with_state(state)
}

async fn health() -> &'static str {
    crate::logging::trace("http.health.checked", &[]);
    "OK"
}

async fn require_crawl_api_key(
    axum::extract::State(_state): axum::extract::State<AppState>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let api_key = std::env::var("CRAWL_API_KEY").ok();
    let value = req.headers().get("x-crawl-api-key").and_then(|v| v.to_str().ok());
    if api_key.is_none() || value != api_key.as_deref() {
        let response = Response::builder()
            .status(StatusCode::UNAUTHORIZED)
            .body(Body::from("Unauthorized"))
            .unwrap();
        return Err(response);
    }

    Ok(next.run(req).await)
}
