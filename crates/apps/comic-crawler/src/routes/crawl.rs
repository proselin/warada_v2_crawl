use std::time::{Duration, Instant};

use anyhow::Result;
use axum::body::Body;
use axum::extract::{Path as AxumPath, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Sse};
use futures_util::StreamExt;
use serde_json::Value;
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::broadcaster::CrawlProgressEvent;
use crate::config::nettruyen_url;
use crate::logging::{trace, trace_error};
use crate::services::nettruyen::{
    download_chapter_image, download_image, extract_comic_detail, fetch_chapter_image_candidates,
    fetch_chapter_list, put_temp_object, rename_to_permanent, timestamp_tag,
};
use crate::state::AppState;

#[derive(Clone, Debug)]
pub struct CrawlResult {
    pub status: u16,
    pub body: String,
}

#[derive(Clone, Debug)]
pub struct RetryResult {
    pub status: u16,
    pub body: String,
}

pub async fn crawl_nettruyen_comic(
    State(state): State<AppState>,
    req: axum::http::Request<Body>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let started_at: Instant = Instant::now();
    let body_bytes: axum::body::Bytes =
        match axum::body::to_bytes(req.into_body(), usize::MAX).await {
            Ok(bytes) => bytes,
            Err(err) => {
                trace_error("crawl.request.rejected.invalid-json", &err, &[]);
                return Err((StatusCode::BAD_REQUEST, "Invalid JSON body".to_string()));
            }
        };

    let payload: Value = match serde_json::from_slice(&body_bytes) {
        Ok(value) => value,
        Err(err) => {
            trace_error("crawl.request.rejected.invalid-json", &err, &[]);
            return Err((StatusCode::BAD_REQUEST, "Invalid JSON body".to_string()));
        }
    };

    let slug_n_id = match payload.get("slug-and-id").and_then(|v| v.as_str()) {
        Some(value) if !value.trim().is_empty() => value.trim().to_string(),
        _ => {
            trace("crawl.request.rejected.invalid-payload", &[]);
            return Err((
                StatusCode::BAD_REQUEST,
                "\"slug-and-id\" must be a non-empty string".to_string(),
            ));
        }
    };

    trace("crawl.request.received", &[("slugNId", slug_n_id.clone())]);
    match state
        .shared
        .database
        .enqueue_crawl_job(&slug_n_id, "comic")
        .await
    {
        Ok(Some(job_id)) => {
            trace(
                "crawl.request.accepted",
                &[
                    ("slugNId", slug_n_id),
                    ("jobId", job_id.to_string()),
                    ("durationMs", started_at.elapsed().as_millis().to_string()),
                ],
            );
            Ok((StatusCode::ACCEPTED, format!("Crawl job {job_id} queued")))
        }
        Ok(None) => Ok((
            StatusCode::CONFLICT,
            "Crawl job already queued or running".to_string(),
        )),
        Err(err) => {
            trace("crawl.queue.enqueue.failed", &[("error", err)]);
            Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "Crawl queue unavailable; apply migrations/0001_init_database.sql".to_string(),
            ))
        }
    }
}

pub async fn retry_failed_chapters(
    State(state): State<AppState>,
    AxumPath(slug): AxumPath<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let started_at = Instant::now();
    trace("crawl.retry.request.received", &[("slug", slug.clone())]);
    match state
        .shared
        .database
        .enqueue_crawl_job(&slug, "retry")
        .await
    {
        Ok(Some(job_id)) => {
            trace(
                "crawl.retry.request.accepted",
                &[
                    ("slug", slug),
                    ("jobId", job_id.to_string()),
                    ("durationMs", started_at.elapsed().as_millis().to_string()),
                ],
            );
            Ok((StatusCode::ACCEPTED, format!("Retry job {job_id} queued")))
        }
        Ok(None) => Ok((
            StatusCode::CONFLICT,
            "Crawl job already queued or running".to_string(),
        )),
        Err(err) => {
            trace("crawl.retry.queue.enqueue.failed", &[("error", err)]);
            Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "Crawl queue unavailable; apply migrations/0001_init_database.sql".to_string(),
            ))
        }
    }
}

pub async fn progress_stream(
    State(state): State<AppState>,
    AxumPath(comic_slug): AxumPath<String>,
) -> Result<
    Sse<
        impl futures_util::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>,
    >,
    StatusCode,
> {
    let has_job = state
        .shared
        .database
        .has_crawl_job(&comic_slug)
        .await
        .unwrap_or(false);
    if !state.comic_exists_by_slug(&comic_slug).await && !state.is_active(&comic_slug) && !has_job {
        trace(
            "crawl.progress.rejected.not-found",
            &[("comicSlug", comic_slug.clone())],
        );
        return Err(StatusCode::NOT_FOUND);
    }

    let rx = state.shared.broadcaster.subscribe(&comic_slug);
    trace(
        "crawl.progress.connected",
        &[("comicSlug", comic_slug.clone())],
    );

    let chapter_events = UnboundedReceiverStream::new(rx).map(move |event| {
        let payload = serde_json::to_string(&event).unwrap();
        Ok(axum::response::sse::Event::default().data(payload))
    });
    let database = state.shared.database.clone();
    let job_events = futures_util::stream::unfold(
        (database, comic_slug, false),
        |(database, slug, finished)| async move {
            if finished {
                return None;
            }
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                if let Ok(Some(job)) = database.latest_crawl_job(&slug).await {
                    if matches!(job.status.as_str(), "finished" | "failed") {
                        let event = CrawlProgressEvent {
                            event_type: job.status.clone(),
                            message: job
                                .error
                                .unwrap_or_else(|| "Crawl job finished".to_string()),
                            data: crate::broadcaster::CrawlProgressData {
                                chapter_id: None,
                                chapter_number: None,
                                status: Some(job.status),
                                comic_slug: slug.clone(),
                            },
                        };
                        let payload = serde_json::to_string(&event).unwrap();
                        return Some((
                            Ok(axum::response::sse::Event::default().data(payload)),
                            (database, slug, true),
                        ));
                    }
                }
            }
        },
    );

    Ok(Sse::new(futures_util::stream::select(
        chapter_events,
        job_events,
    )))
}

pub(crate) async fn run_crawl_queue_worker(state: AppState) {
    let worker_id = format!(
        "{}-{}",
        std::env::var("HOSTNAME").unwrap_or_else(|_| "crawler".to_string()),
        std::process::id()
    );

    loop {
        match state.shared.database.claim_crawl_job(&worker_id).await {
            Ok(Some(job)) => {
                trace(
                    "crawl.queue.claimed",
                    &[
                        ("jobId", job.id.to_string()),
                        ("slug", job.slug.clone()),
                        ("jobType", job.job_type.clone()),
                    ],
                );
                let result = match job.job_type.as_str() {
                    "comic" if state.try_mark_active(&job.slug) => {
                        match crawl_nettruyen_comic_impl(&state, &job.slug).await {
                            Ok(result) if result.status == 200 => Ok(()),
                            Ok(result) => Err(result.body),
                            Err(err) => Err(err.to_string()),
                        }
                    }
                    "comic" => Err("comic crawl is already active on this pod".to_string()),
                    "retry" => match retry_failed_chapters_impl(&state, &job.slug).await {
                        Ok(result) if result.status == 200 => Ok(()),
                        Ok(result) => Err(result.body),
                        Err(err) => Err(err.to_string()),
                    },
                    _ => Err(format!("unsupported crawl job type: {}", job.job_type)),
                };
                let (success, error) = match result {
                    Ok(()) => (true, None),
                    Err(err) => (false, Some(err)),
                };
                if let Err(err) = state
                    .shared
                    .database
                    .finish_crawl_job(job.id, &worker_id, success, error.as_deref())
                    .await
                {
                    tracing::error!(job_id = job.id, error = %err, "crawl_job_completion_failed");
                }
                trace(
                    "crawl.queue.finished",
                    &[
                        ("jobId", job.id.to_string()),
                        ("success", success.to_string()),
                    ],
                );
            }
            Ok(None) => tokio::time::sleep(Duration::from_secs(1)).await,
            Err(err) => {
                tracing::error!(error = %err, "crawl_queue_poll_failed");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }
}

async fn crawl_nettruyen_comic_impl(state: &AppState, slug_n_id: &str) -> Result<CrawlResult> {
    let started_at = Instant::now();
    if state.comic_exists_by_origin_path(slug_n_id).await {
        trace(
            "crawl.rejected.duplicate",
            &[("slugNId", slug_n_id.to_string())],
        );
        return Ok(CrawlResult {
            status: 409,
            body: "Conflict".to_string(),
        });
    }

    let mut resolved_slug: Option<String> = None;
    let result: Result<CrawlResult> = async {
        let detail = extract_comic_detail(slug_n_id).await?;
        resolved_slug = Some(detail.slug.clone());
        state.mark_active(&detail.slug);

        let chapter_stubs = fetch_chapter_list(&detail.slug, &detail.comic_id).await?;
        let tag_ids = state.resolve_or_create_tags(&detail.genres).await;
        trace(
            "crawl.metadata.fetched",
            &[
                ("slug", detail.slug.clone()),
                ("chapterCount", chapter_stubs.len().to_string()),
                ("tagCount", tag_ids.len().to_string()),
                ("hasThumbnail", detail.thumbnail_url.is_some().to_string()),
            ],
        );

        let mut thumb_image_id = None;
        if let Some(url) = &detail.thumbnail_url {
            let result = pull_and_store_image(url, "thumbnail", &detail.slug).await?;
            let image_id = state.next_id("image_id_seq").await;
            let image = crate::state::ImageRecord {
                id: image_id,
                chapter_id: None,
                position: None,
                file_name: result.1,
                file_path: result.0,
                origin_url: Some(url.clone()),
                image_type: "THUMB".to_string(),
            };
            state.insert_image(image).await;
            thumb_image_id = Some(image_id);
        }

        let comic_id = state.next_id("comic_id_seq").await;
        let comic = crate::state::ComicRecord {
            id: comic_id,
            slug: detail.slug.clone(),
            title: detail.title.clone(),
            author: detail.author.clone(),
            description: detail.description.clone(),
            status: "OnGoing".to_string(),
            chapter_count: Some(chapter_stubs.len() as i32),
            crawling_status: "9999".to_string(),
            origin_id: Some(detail.comic_id.clone()),
            origin_url: Some(format!("{}/truyen-tranh/{}", nettruyen_url(), slug_n_id)),
            origin_path_params: Some(slug_n_id.to_string()),
            thumb_image_id,
        };
        state.insert_comic(comic).await;
        for tag_id in &tag_ids {
            state.link_comic_tag(comic_id, *tag_id).await;
        }
        for stub in &chapter_stubs {
            let chapter_id = state.next_id("chapter_id_seq").await;
            let chapter = crate::state::ChapterRecord {
                id: chapter_id,
                comic_id,
                chapter_num: stub.chapter_name.clone(),
                position: stub.position,
                crawling_status: "0000".to_string(),
                origin_url: stub.origin_url.clone(),
                origin_path_params: stub.origin_path_params.clone(),
            };
            state.insert_chapter(chapter).await;
        }
        state.increment_comic_count(&tag_ids).await;
        trace(
            "crawl.metadata.persisted",
            &[
                ("slug", detail.slug.clone()),
                ("comicId", comic_id.to_string()),
                ("chapterCount", chapter_stubs.len().to_string()),
            ],
        );

        let inserted = state.get_chapters_for_comic(comic_id);
        run_chapter_crawl_loop(state, &detail.slug, &inserted).await;
        trace(
            "crawl.completed",
            &[
                ("slug", detail.slug.clone()),
                ("comicId", comic_id.to_string()),
                (
                    "durationMs",
                    format!("{}", started_at.elapsed().as_millis()),
                ),
            ],
        );
        Ok(CrawlResult {
            status: 200,
            body: "Success".to_string(),
        })
    }
    .await;

    if let Err(err) = &result {
        trace_error(
            "crawl.failed",
            &err,
            &[
                ("slugNId", slug_n_id.to_string()),
                ("resolvedSlug", resolved_slug.clone().unwrap_or_default()),
                (
                    "durationMs",
                    format!("{}", started_at.elapsed().as_millis()),
                ),
            ],
        );
    }

    state.mark_inactive(slug_n_id);
    if let Some(slug) = resolved_slug.as_deref() {
        state.mark_inactive(slug);
    }
    trace(
        "crawl.inactive",
        &[
            ("slugNId", slug_n_id.to_string()),
            ("resolvedSlug", resolved_slug.unwrap_or_default()),
        ],
    );
    result
}

async fn retry_failed_chapters_impl(state: &AppState, slug: &str) -> Result<RetryResult> {
    let started_at = Instant::now();
    let comic = if state.shared.database.is_enabled() {
        state.shared.database.get_comic_by_slug(slug).await
    } else {
        state.shared.comics.read().unwrap().get(slug).cloned()
    };

    let Some(comic) = comic else {
        trace(
            "crawl.retry.rejected.not-found",
            &[
                ("slug", slug.to_string()),
                (
                    "durationMs",
                    format!("{}", started_at.elapsed().as_millis()),
                ),
            ],
        );
        return Ok(RetryResult {
            status: 404,
            body: "Not Found".to_string(),
        });
    };

    if !state.try_mark_active(slug) {
        trace(
            "crawl.retry.rejected.active",
            &[
                ("slug", slug.to_string()),
                ("comicId", comic.id.to_string()),
                (
                    "durationMs",
                    format!("{}", started_at.elapsed().as_millis()),
                ),
            ],
        );
        return Ok(RetryResult {
            status: 409,
            body: "Conflict".to_string(),
        });
    }

    let result = async {
        let pending = if state.shared.database.is_enabled() {
            state
                .shared
                .database
                .get_pending_chapters_for_comic(comic.id)
                .await
                .unwrap_or_default()
        } else {
            state
                .shared
                .chapters
                .read()
                .unwrap()
                .values()
                .filter(|chapter| chapter.comic_id == comic.id && chapter.crawling_status == "0000")
                .cloned()
                .collect::<Vec<_>>()
        };
        trace(
            "crawl.retry.started",
            &[
                ("slug", slug.to_string()),
                ("comicId", comic.id.to_string()),
                ("chapterCount", pending.len().to_string()),
            ],
        );
        run_chapter_crawl_loop(state, slug, &pending).await;
        trace(
            "crawl.retry.completed",
            &[
                ("slug", slug.to_string()),
                ("comicId", comic.id.to_string()),
                ("chapterCount", pending.len().to_string()),
                (
                    "durationMs",
                    format!("{}", started_at.elapsed().as_millis()),
                ),
            ],
        );
        Ok(RetryResult {
            status: 200,
            body: "Retry queued".to_string(),
        })
    }
    .await;

    state.mark_inactive(slug);
    trace("crawl.retry.inactive", &[("slug", slug.to_string())]);
    result
}

async fn run_chapter_crawl_loop(
    state: &AppState,
    comic_slug: &str,
    chapter_rows: &[crate::state::ChapterRecord],
) {
    let started_at = Instant::now();
    trace(
        "crawl.chapters.started",
        &[
            ("comicSlug", comic_slug.to_string()),
            ("chapterCount", chapter_rows.len().to_string()),
        ],
    );

    const CONCURRENCY: usize = 4;
    let mut chapters = chapter_rows.iter().cloned();
    let mut tasks = tokio::task::JoinSet::new();

    for chapter in chapters.by_ref().take(CONCURRENCY) {
        let state = state.clone();
        let comic_slug = comic_slug.to_string();
        tasks.spawn(async move { crawl_chapter(state, comic_slug, chapter).await });
    }

    while let Some(result) = tasks.join_next().await {
        if let Err(err) = result {
            trace_error(
                "crawl.chapter.task_failed",
                &err,
                &[("comicSlug", comic_slug.to_string())],
            );
        }
        if let Some(chapter) = chapters.next() {
            let state = state.clone();
            let comic_slug = comic_slug.to_string();
            tasks.spawn(async move { crawl_chapter(state, comic_slug, chapter).await });
        }
    }

    trace(
        "crawl.chapters.completed",
        &[
            ("comicSlug", comic_slug.to_string()),
            ("chapterCount", chapter_rows.len().to_string()),
            (
                "durationMs",
                format!("{}", started_at.elapsed().as_millis()),
            ),
        ],
    );
}

async fn crawl_chapter(state: AppState, comic_slug: String, chapter: crate::state::ChapterRecord) {
    let chapter_started_at = Instant::now();
    trace(
        "crawl.chapter.started",
        &[
            ("comicSlug", comic_slug.clone()),
            ("chapterId", chapter.id.to_string()),
            ("chapterNumber", chapter.chapter_num.clone()),
        ],
    );
    let event = CrawlProgressEvent {
        event_type: "processing".to_string(),
        message: format!("Crawling chapter {}", chapter.chapter_num),
        data: crate::broadcaster::CrawlProgressData {
            chapter_id: Some(chapter.id),
            chapter_number: Some(chapter.chapter_num.clone()),
            status: None,
            comic_slug: comic_slug.clone(),
        },
    };
    state.shared.broadcaster.broadcast(&comic_slug, &event);

    let mut image_count = 0usize;
    if let Err(err) = async {
        if chapter.origin_url.is_empty() {
            return Err(anyhow::anyhow!("Chapter has no originUrl"));
        }
        let candidates = fetch_chapter_image_candidates(&chapter.origin_url).await?;
        let permanent_dir = format!("{}/chapters/{}", comic_slug, chapter.chapter_num);
        let permanent_dir_for_async = permanent_dir.clone();
        let downloaded =
            futures_util::future::try_join_all(candidates.into_iter().enumerate().map(
                move |(position, candidate)| {
                    let permanent_dir = permanent_dir_for_async.clone();
                    async move {
                        let image = download_chapter_image(&candidate).await?;
                        let file_name =
                            format!("{}_{}{}", timestamp_tag(), position, image.extension);
                        let temp_path =
                            put_temp_object(&file_name, &image.buffer, &image.content_type).await?;
                        let file_path = format!("{}/{}", permanent_dir, file_name);
                        rename_to_permanent(&temp_path, &file_path).await?;
                        Ok::<(i32, String, String, String), anyhow::Error>((
                            position as i32,
                            file_name,
                            file_path,
                            candidate.sv1.clone(),
                        ))
                    }
                },
            ))
            .await?;

        image_count = downloaded.len();
        for (position, file_name, file_path, origin_url) in downloaded {
            let image_id = state.next_id("image_id_seq").await;
            let image = crate::state::ImageRecord {
                id: image_id,
                chapter_id: Some(chapter.id),
                position: Some(position),
                file_name,
                file_path,
                origin_url: Some(origin_url),
                image_type: "CHAPTER_IMAGE".to_string(),
            };
            state.insert_image(image).await;
        }
        state.update_chapter_status(chapter.id, "9999").await;
        Ok::<(), anyhow::Error>(())
    }
    .await
    {
        trace_error(
            "crawl.chapter.failed",
            &err,
            &[
                ("comicSlug", comic_slug.clone()),
                ("chapterId", chapter.id.to_string()),
                ("chapterNumber", chapter.chapter_num.clone()),
                (
                    "durationMs",
                    format!("{}", chapter_started_at.elapsed().as_millis()),
                ),
            ],
        );
        let event = CrawlProgressEvent {
            event_type: "failed".to_string(),
            message: err.to_string(),
            data: crate::broadcaster::CrawlProgressData {
                chapter_id: Some(chapter.id),
                chapter_number: Some(chapter.chapter_num.clone()),
                status: None,
                comic_slug: comic_slug.clone(),
            },
        };
        state.shared.broadcaster.broadcast(&comic_slug, &event);
    }

    trace(
        "crawl.chapter.completed",
        &[
            ("comicSlug", comic_slug.clone()),
            ("chapterId", chapter.id.to_string()),
            ("imageCount", image_count.to_string()),
            (
                "durationMs",
                format!("{}", chapter_started_at.elapsed().as_millis()),
            ),
        ],
    );
    let event = CrawlProgressEvent {
        event_type: "completed".to_string(),
        message: format!("Chapter {} completed", chapter.chapter_num),
        data: crate::broadcaster::CrawlProgressData {
            chapter_id: Some(chapter.id),
            chapter_number: Some(chapter.chapter_num),
            status: None,
            comic_slug: comic_slug.clone(),
        },
    };
    state.shared.broadcaster.broadcast(&comic_slug, &event);
}

async fn pull_and_store_image(
    url: &str,
    unique_name: &str,
    permanent_dir: &str,
) -> Result<(String, String)> {
    let img = download_image(url).await?;
    let file_name = format!("{}_{}{}", timestamp_tag(), unique_name, img.extension);
    let temp_path = put_temp_object(&file_name, &img.buffer, &img.content_type).await?;
    let file_path = format!("{}/{}", permanent_dir, file_name);
    rename_to_permanent(&temp_path, &file_path).await?;
    Ok((file_path, file_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::ChapterRecord;
    use std::{
        io::{Read, Write},
        net::TcpListener,
        thread,
    };

    fn mock_server(responses: Vec<(u16, &'static str, &'static str)>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let base = format!("http://{address}");
        thread::spawn(move || {
            for (status, content_type, body) in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let mut request = [0; 2048];
                let _ = stream.read(&mut request).unwrap();
                let reason = if status == 200 {
                    "OK"
                } else {
                    "Internal Server Error"
                };
                write!(
                    stream,
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                )
                .unwrap();
            }
        });
        base
    }

    struct RestoreTestEnv {
        nettruyen_url: Option<std::ffi::OsString>,
        database_path: Option<std::ffi::OsString>,
    }

    impl Drop for RestoreTestEnv {
        fn drop(&mut self) {
            unsafe {
                if let Some(value) = self.nettruyen_url.take() {
                    std::env::set_var("NETTRUYEN_URL", value);
                } else {
                    std::env::remove_var("NETTRUYEN_URL");
                }
                if let Some(value) = self.database_path.take() {
                    std::env::set_var("DATABASE_PATH", value);
                } else {
                    std::env::remove_var("DATABASE_PATH");
                }
            }
        }
    }

    #[tokio::test]
    async fn queued_requests_retry_and_progress_cover_memory_and_sqlite_paths() {
        let _env_lock = crate::test_support::ENV_LOCK.lock().await;
        let previous = std::env::var("DATABASE_PATH").ok();
        let path = std::env::temp_dir().join(format!(
            "warada-routes-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        unsafe {
            std::env::set_var("DATABASE_PATH", &path);
        }
        let state = AppState::default();
        state.shared.database.initialize().await.unwrap();

        let request = |body: &'static str| {
            axum::http::Request::builder()
                .body(Body::from(body))
                .unwrap()
        };
        assert_eq!(
            crawl_nettruyen_comic(State(state.clone()), request("{"))
                .await
                .err()
                .unwrap()
                .0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            crawl_nettruyen_comic(State(state.clone()), request("{}"))
                .await
                .err()
                .unwrap()
                .0,
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            crawl_nettruyen_comic(
                State(state.clone()),
                request(r#"{"slug-and-id":"queued-comic-1"}"#),
            )
            .await
            .unwrap()
            .into_response()
            .status(),
            StatusCode::ACCEPTED
        );
        assert_eq!(
            crawl_nettruyen_comic(
                State(state.clone()),
                request(r#"{"slug-and-id":"queued-comic-1"}"#),
            )
            .await
            .unwrap()
            .into_response()
            .status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            retry_failed_chapters(State(state.clone()), AxumPath("missing".to_owned()),)
                .await
                .unwrap()
                .into_response()
                .status(),
            StatusCode::ACCEPTED
        );

        let failed = state
            .shared
            .database
            .claim_crawl_job("test-worker")
            .await
            .unwrap()
            .unwrap();
        state
            .shared
            .database
            .finish_crawl_job(failed.id, "test-worker", false, Some("failed"))
            .await
            .unwrap();
        let stream = progress_stream(State(state.clone()), AxumPath(failed.slug.clone()))
            .await
            .unwrap()
            .into_response();
        let mut body = stream.into_body().into_data_stream();
        let event = tokio::time::timeout(Duration::from_secs(2), body.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert!(String::from_utf8_lossy(&event).contains("failed"));
        assert_eq!(
            progress_stream(State(state.clone()), AxumPath("unknown".to_owned()))
                .await
                .unwrap_err(),
            StatusCode::NOT_FOUND
        );

        drop(body);
        unsafe {
            if let Some(previous) = previous {
                std::env::set_var("DATABASE_PATH", previous);
            } else {
                std::env::remove_var("DATABASE_PATH");
            }
        }
        drop(state);
        tokio::time::sleep(Duration::from_millis(20)).await;
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", path.display()));
        }
    }

    #[tokio::test]
    async fn retries_existing_comics_and_broadcasts_failed_chapter_events() {
        let state = AppState::default();
        let missing = retry_failed_chapters_impl(&state, "missing").await.unwrap();
        assert_eq!(missing.status, 404);

        state
            .insert_comic(crate::state::ComicRecord {
                id: 1,
                slug: "retry-comic".to_owned(),
                title: "Retry".to_owned(),
                author: None,
                description: None,
                status: "ongoing".to_owned(),
                chapter_count: Some(0),
                crawling_status: "done".to_owned(),
                origin_id: None,
                origin_url: None,
                origin_path_params: None,
                thumb_image_id: None,
            })
            .await;
        state.mark_active("retry-comic");
        assert_eq!(
            retry_failed_chapters_impl(&state, "retry-comic")
                .await
                .unwrap()
                .status,
            409
        );
        state.mark_inactive("retry-comic");
        assert_eq!(
            retry_failed_chapters_impl(&state, "retry-comic")
                .await
                .unwrap()
                .status,
            200
        );

        let mut events = state.shared.broadcaster.subscribe("retry-comic");
        let chapters: Vec<_> = (0..5)
            .map(|id| ChapterRecord {
                id,
                comic_id: 1,
                chapter_num: id.to_string(),
                position: id as i32,
                crawling_status: "pending".to_owned(),
                origin_url: String::new(),
                origin_path_params: String::new(),
            })
            .collect();
        run_chapter_crawl_loop(&state, "retry-comic", &chapters).await;
        let mut received = Vec::new();
        for _ in 0..chapters.len() * 3 {
            received.push(events.try_recv().unwrap().event_type);
        }
        assert_eq!(
            received
                .iter()
                .filter(|event| *event == "processing")
                .count(),
            5
        );
        assert_eq!(
            received.iter().filter(|event| *event == "failed").count(),
            5
        );
        assert_eq!(
            received
                .iter()
                .filter(|event| *event == "completed")
                .count(),
            5
        );
    }

    #[tokio::test]
    async fn comic_crawl_persists_metadata_and_finishes_empty_chapters() {
        let _env_lock = crate::test_support::ENV_LOCK.lock().await;
        let _restore = RestoreTestEnv {
            nettruyen_url: std::env::var_os("NETTRUYEN_URL"),
            database_path: std::env::var_os("DATABASE_PATH"),
        };
        let base = mock_server(vec![
            (
                200,
                "text/html",
                r#"<script>gOpts.comicSlug = "crawl-test"; gOpts.comicId = "42";</script><script type="application/ld+json">{"@type":"ComicSeries","name":"Crawl Test","genre":["Action"]}</script>"#,
            ),
            (
                200,
                "application/json",
                r#"{"data":[{"chapter_slug":"chapter-1","chapter_id":"1","chapter_name":"Chapter 1","chapter_num":1}]}"#,
            ),
            (200, "text/html", "<html></html>"),
        ]);
        unsafe {
            std::env::set_var("NETTRUYEN_URL", &base);
            std::env::remove_var("DATABASE_PATH");
        }

        let state = AppState::default();
        let result = crawl_nettruyen_comic_impl(&state, "crawl-test-42")
            .await
            .unwrap();

        assert_eq!(result.status, 200);
        assert!(state.comic_exists_by_slug("crawl-test").await);
        assert_eq!(state.shared.tags.read().unwrap()["action"].comic_count, 1);
        let chapters = state.get_chapters_for_comic(1);
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].crawling_status, "9999");
        assert_eq!(
            crawl_nettruyen_comic_impl(&state, "crawl-test-42")
                .await
                .unwrap()
                .status,
            409
        );
    }
}
