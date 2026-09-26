use std::time::Instant;

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
    let started_at = Instant::now();
    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
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
            return Err((StatusCode::BAD_REQUEST, "\"slug-and-id\" must be a non-empty string".to_string()));
        }
    };

    trace("crawl.request.received", &[("slugNId", slug_n_id.clone())]);

    let result = crawl_nettruyen_comic_impl(&state, &slug_n_id).await;
    match &result {
        Ok(r) => trace("crawl.request.completed", &[("slugNId", slug_n_id.clone()), ("status", r.status.to_string())]),
        Err(err) => trace_error("crawl.request.failed", &err, &[("slugNId", slug_n_id.clone()), ("durationMs", format!("{}", started_at.elapsed().as_millis()))]),
    }

    result.map(|item| (StatusCode::from_u16(item.status).unwrap(), item.body)).map_err(|err| {
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        (status, err.to_string())
    })
}

pub async fn retry_failed_chapters(
    State(state): State<AppState>,
    AxumPath(slug): AxumPath<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let started_at = Instant::now();
    trace("crawl.retry.request.received", &[("slug", slug.clone())]);
    let result = retry_failed_chapters_impl(&state, &slug).await;
    match &result {
        Ok(r) => trace("crawl.retry.request.completed", &[("slug", slug.clone()), ("status", r.status.to_string())]),
        Err(err) => trace_error("crawl.retry.request.failed", &err, &[("slug", slug.clone()), ("durationMs", format!("{}", started_at.elapsed().as_millis()))]),
    }
    result.map(|item| (StatusCode::from_u16(item.status).unwrap(), item.body)).map_err(|err| {
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        (status, err.to_string())
    })
}

pub async fn progress_stream(
    State(state): State<AppState>,
    AxumPath(comic_slug): AxumPath<String>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>>, StatusCode> {
    if !state.comic_exists_by_slug(&comic_slug) {
        trace("crawl.progress.rejected.not-found", &[("comicSlug", comic_slug.clone())]);
        return Err(StatusCode::NOT_FOUND);
    }

    let rx = state.shared.broadcaster.subscribe(&comic_slug);
    trace("crawl.progress.connected", &[("comicSlug", comic_slug.clone())]);

    let stream = UnboundedReceiverStream::new(rx).map(move |event| {
        let payload = serde_json::to_string(&event).unwrap();
        Ok(axum::response::sse::Event::default().data(payload))
    });

    Ok(Sse::new(stream))
}

async fn crawl_nettruyen_comic_impl(state: &AppState, slug_n_id: &str) -> Result<CrawlResult> {
    let started_at = Instant::now();
    if state.is_active(slug_n_id) {
        trace("crawl.rejected.active", &[("slugNId", slug_n_id.to_string()), ("durationMs", format!("{}", started_at.elapsed().as_millis()))]);
        return Ok(CrawlResult { status: 409, body: "Conflict".to_string() });
    }

    if state.comic_exists_by_origin_path(slug_n_id) {
        trace("crawl.rejected.duplicate", &[("slugNId", slug_n_id.to_string())]);
        return Ok(CrawlResult { status: 409, body: "Conflict".to_string() });
    }

    state.mark_active(slug_n_id);
    let mut resolved_slug: Option<String> = None;
    let result = async {
        let detail = extract_comic_detail(slug_n_id).await?;
        resolved_slug = Some(detail.slug.clone());
        state.mark_active(&detail.slug);

        let chapter_stubs = fetch_chapter_list(&detail.slug, &detail.comic_id).await?;
        let tag_ids = state.resolve_or_create_tags(&detail.genres);
        trace("crawl.metadata.fetched", &[
            ("slug", detail.slug.clone()),
            ("chapterCount", chapter_stubs.len().to_string()),
            ("tagCount", tag_ids.len().to_string()),
            ("hasThumbnail", detail.thumbnail_url.is_some().to_string()),
        ]);

        let mut thumb_image_id = None;
        if let Some(url) = &detail.thumbnail_url {
            let result = pull_and_store_image(url, "thumbnail", &detail.slug).await?;
            let image_id = state.next_id("image_id_seq");
            let image = crate::state::ImageRecord {
                id: image_id,
                chapter_id: None,
                position: None,
                file_name: result.1,
                file_path: result.0,
                origin_url: Some(url.clone()),
                image_type: "THUMB".to_string(),
            };
            state.insert_image(image);
            thumb_image_id = Some(image_id);
        }

        let comic_id = state.next_id("comic_id_seq");
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
        state.insert_comic(comic);
        for tag_id in &tag_ids {
            state.link_comic_tag(comic_id, *tag_id);
        }
        for stub in &chapter_stubs {
            let chapter_id = state.next_id("chapter_id_seq");
            let chapter = crate::state::ChapterRecord {
                id: chapter_id,
                comic_id,
                chapter_num: stub.chapter_name.clone(),
                position: stub.position,
                crawling_status: "0000".to_string(),
                origin_url: stub.origin_url.clone(),
                origin_path_params: stub.origin_path_params.clone(),
            };
            state.insert_chapter(chapter);
        }
        state.increment_comic_count(&tag_ids);
        trace("crawl.metadata.persisted", &[("slug", detail.slug.clone()), ("comicId", comic_id.to_string()), ("chapterCount", chapter_stubs.len().to_string())]);

        let inserted = state.get_chapters_for_comic(comic_id);
        run_chapter_crawl_loop(&state, &detail.slug, &inserted).await;
        trace("crawl.completed", &[("slug", detail.slug.clone()), ("comicId", comic_id.to_string()), ("durationMs", format!("{}", started_at.elapsed().as_millis()))]);
        Ok(CrawlResult { status: 200, body: "Success".to_string() })
    }
    .await;

    if let Err(err) = &result {
        trace_error("crawl.failed", &err, &[("slugNId", slug_n_id.to_string()), ("resolvedSlug", resolved_slug.clone().unwrap_or_default()), ("durationMs", format!("{}", started_at.elapsed().as_millis()))]);
    }

    state.mark_inactive(slug_n_id);
    if let Some(slug) = resolved_slug.as_deref() {
        state.mark_inactive(slug);
    }
    trace("crawl.inactive", &[("slugNId", slug_n_id.to_string()), ("resolvedSlug", resolved_slug.clone().unwrap_or_default())]);
    result
}

async fn retry_failed_chapters_impl(state: &AppState, slug: &str) -> Result<RetryResult> {
    let started_at = Instant::now();
    let comic = {
        let comics = state.shared.comics.read().unwrap();
        comics.get(slug).cloned()
    };

    let Some(comic) = comic else {
        trace("crawl.retry.rejected.not-found", &[("slug", slug.to_string()), ("durationMs", format!("{}", started_at.elapsed().as_millis()))]);
        return Ok(RetryResult { status: 404, body: "Not Found".to_string() });
    };

    if state.is_active(slug) {
        trace("crawl.retry.rejected.active", &[("slug", slug.to_string()), ("comicId", comic.id.to_string()), ("durationMs", format!("{}", started_at.elapsed().as_millis()))]);
        return Ok(RetryResult { status: 409, body: "Conflict".to_string() });
    }

    state.mark_active(slug);
    let result = async {
        let pending = state
            .shared
            .chapters
            .read()
            .unwrap()
            .values()
            .filter(|chapter| chapter.comic_id == comic.id && chapter.crawling_status == "0000")
            .cloned()
            .collect::<Vec<_>>();
        trace("crawl.retry.started", &[("slug", slug.to_string()), ("comicId", comic.id.to_string()), ("chapterCount", pending.len().to_string())]);
        run_chapter_crawl_loop(state, slug, &pending).await;
        trace("crawl.retry.completed", &[("slug", slug.to_string()), ("comicId", comic.id.to_string()), ("chapterCount", pending.len().to_string()), ("durationMs", format!("{}", started_at.elapsed().as_millis()))]);
        Ok(RetryResult { status: 200, body: "Retry queued".to_string() })
    }
    .await;

    state.mark_inactive(slug);
    trace("crawl.retry.inactive", &[("slug", slug.to_string())]);
    result
}

async fn run_chapter_crawl_loop(state: &AppState, comic_slug: &str, chapter_rows: &[crate::state::ChapterRecord]) {
    let started_at = Instant::now();
    trace("crawl.chapters.started", &[("comicSlug", comic_slug.to_string()), ("chapterCount", chapter_rows.len().to_string())]);

    for chapter in chapter_rows {
        let chapter_started_at = Instant::now();
        trace("crawl.chapter.started", &[("comicSlug", comic_slug.to_string()), ("chapterId", chapter.id.to_string()), ("chapterNumber", chapter.chapter_num.clone())]);
        let event = CrawlProgressEvent {
            event_type: "processing".to_string(),
            message: format!("Crawling chapter {}", chapter.chapter_num),
            data: crate::broadcaster::CrawlProgressData {
                chapter_id: chapter.id,
                chapter_number: chapter.chapter_num.clone(),
                status: None,
                comic_slug: comic_slug.to_string(),
            },
        };
        state.shared.broadcaster.broadcast(comic_slug, &event);

        let mut image_count = 0usize;
        if let Err(err) = async {
            if chapter.origin_url.is_empty() {
                return Err(anyhow::anyhow!("Chapter has no originUrl"));
            }
            let candidates = fetch_chapter_image_candidates(&chapter.origin_url).await?;
            let permanent_dir = format!("{}/chapters/{}", comic_slug, chapter.chapter_num);
            let permanent_dir_for_async = permanent_dir.clone();
            let downloaded = futures_util::future::try_join_all(candidates.into_iter().enumerate().map(move |(position, candidate)| {
                let permanent_dir = permanent_dir_for_async.clone();
                async move {
                    let image = download_chapter_image(&candidate).await?;
                    let file_name = format!("{}_{}{}", timestamp_tag(), position, image.extension);
                    let temp_path = put_temp_object(&file_name, &image.buffer, &image.content_type).await?;
                    let file_path = format!("{}/{}", permanent_dir, file_name);
                    rename_to_permanent(&temp_path, &file_path).await?;
                    Ok::<(i32, String, String, String), anyhow::Error>((position as i32, file_name, file_path, candidate.sv1.clone()))
                }
            }))
            .await?;

            image_count = downloaded.len();
            for (position, file_name, file_path, origin_url) in downloaded {
                let image_id = state.next_id("image_id_seq");
                let image = crate::state::ImageRecord {
                    id: image_id,
                    chapter_id: Some(chapter.id),
                    position: Some(position),
                    file_name,
                    file_path,
                    origin_url: Some(origin_url),
                    image_type: "CHAPTER_IMAGE".to_string(),
                };
                state.insert_image(image);
            }
            state.update_chapter_status(chapter.id, "9999");
            Ok::<(), anyhow::Error>(())
        }
        .await
        {
            let err = err;
            trace_error("crawl.chapter.failed", &err, &[("comicSlug", comic_slug.to_string()), ("chapterId", chapter.id.to_string()), ("chapterNumber", chapter.chapter_num.clone()), ("durationMs", format!("{}", chapter_started_at.elapsed().as_millis()))]);
            let event = CrawlProgressEvent {
                event_type: "failed".to_string(),
                message: err.to_string(),
                data: crate::broadcaster::CrawlProgressData {
                    chapter_id: chapter.id,
                    chapter_number: chapter.chapter_num.clone(),
                    status: None,
                    comic_slug: comic_slug.to_string(),
                },
            };
            state.shared.broadcaster.broadcast(comic_slug, &event);
        }

        trace("crawl.chapter.completed", &[("comicSlug", comic_slug.to_string()), ("chapterId", chapter.id.to_string()), ("imageCount", image_count.to_string()), ("durationMs", format!("{}", chapter_started_at.elapsed().as_millis()))]);
        let event = CrawlProgressEvent {
            event_type: "completed".to_string(),
            message: format!("Chapter {} completed", chapter.chapter_num),
            data: crate::broadcaster::CrawlProgressData {
                chapter_id: chapter.id,
                chapter_number: chapter.chapter_num.clone(),
                status: None,
                comic_slug: comic_slug.to_string(),
            },
        };
        state.shared.broadcaster.broadcast(comic_slug, &event);
    }

    trace("crawl.chapters.completed", &[("comicSlug", comic_slug.to_string()), ("chapterCount", chapter_rows.len().to_string()), ("durationMs", format!("{}", started_at.elapsed().as_millis()))]);
}

async fn pull_and_store_image(url: &str, unique_name: &str, permanent_dir: &str) -> Result<(String, String)> {
    let img = download_image(url).await?;
    let file_name = format!("{}_{}{}", timestamp_tag(), unique_name, img.extension);
    let temp_path = put_temp_object(&file_name, &img.buffer, &img.content_type).await?;
    let file_path = format!("{}/{}", permanent_dir, file_name);
    rename_to_permanent(&temp_path, &file_path).await?;
    Ok((file_path, file_name))
}
