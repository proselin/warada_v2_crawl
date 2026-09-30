use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post};
use axum::Router;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use sysinfo::{Pid, ProcessesToUpdate, System};
use tracing::Instrument;

use crate::routes::crawl::{crawl_nettruyen_comic, progress_stream, retry_failed_chapters};
use crate::state::AppState;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

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
        .layer(middleware::from_fn(log_request_performance))
        .with_state(state)
}

async fn log_request_performance(req: Request<Body>, next: Next) -> Response {
    let request_id: u64 = REQUEST_ID.fetch_add(1, Ordering::Relaxed);
    let method: String = req.method().to_string();
    let path: String = req.uri().path().to_string();
    let started_at: Instant = Instant::now();
    let mut system: System = System::new();
    let pid: Pid = Pid::from_u32(std::process::id());
    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    let memory_before_bytes = system.process(pid).map(|process| process.memory());
    let cpu_before = process_cpu_time_micros();
    let span = tracing::info_span!("http.request", request_id, method, path);

    let response = crate::logging::REQUEST_ID
        .scope(request_id, next.run(req).instrument(span.clone()))
        .await;

    system.refresh_processes(ProcessesToUpdate::Some(&[pid]), true);
    let memory_after_bytes = system.process(pid).map(|process| process.memory());
    let cpu_after = process_cpu_time_micros();
    span.in_scope(|| {
        tracing::info!(
            target: "performance",
            event = "http.request.completed",
            request_id,
            status = response.status().as_u16(),
            response_duration_ms = started_at.elapsed().as_millis() as u64,
            process_cpu_delta_ms = cpu_before.zip(cpu_after).map(|(before, after)| after.saturating_sub(before) / 1_000),
            process_memory_before_bytes = memory_before_bytes,
            process_memory_after_bytes = memory_after_bytes,
            process_memory_delta_bytes = memory_before_bytes.zip(memory_after_bytes).map(|(before, after)| after as i64 - before as i64),
            "request performance summary"
        );
    });
    response
}

fn process_cpu_time_micros() -> Option<u64> {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::zeroed();
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) } != 0 {
        return None;
    }
    let usage = unsafe { usage.assume_init() };
    let user = usage.ru_utime.tv_sec as u64 * 1_000_000 + usage.ru_utime.tv_usec as u64;
    let system = usage.ru_stime.tv_sec as u64 * 1_000_000 + usage.ru_stime.tv_usec as u64;
    Some(user + system)
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
