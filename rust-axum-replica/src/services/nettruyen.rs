use std::fs;

use anyhow::{Context, Result};
use chrono::{Datelike, Timelike};
use regex::Regex;
use serde_json::Value;

use crate::config::{image_storage_dir, nettruyen_url};

#[derive(Clone, Debug)]
pub struct ExtractedComicDetail {
    pub slug: String,
    pub comic_id: String,
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub genres: Vec<String>,
    pub thumbnail_url: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ExtractedChapterStub {
    pub chapter_id: String,
    pub chapter_name: String,
    pub position: i32,
    pub origin_path_params: String,
    pub origin_url: String,
}

#[derive(Clone, Debug)]
pub struct DownloadedImage {
    pub buffer: Vec<u8>,
    pub content_type: String,
    pub extension: String,
}

#[derive(Clone, Debug)]
pub struct ImageCandidate {
    pub sv1: String,
    pub sv2: Option<String>,
}

pub async fn extract_comic_detail(slug_n_id: &str) -> Result<ExtractedComicDetail> {
    let url = format!("{}/truyen-tranh/{}", nettruyen_url(), slug_n_id);
    let response = reqwest::get(&url).await.context("fetch comic page")?;
    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch comic page {}: HTTP {}", url, response.status());
    }
    let html = response.text().await?;

    let slug = extract_regex_group(&html, r#"gOpts\.comicSlug\s*=\s*['\"]([^'\"]+)['\"]"#, "comicSlug")?;
    let comic_id = extract_regex_group(&html, r#"gOpts\.comicId\s*=\s*['\"]?(\d+)['\"]?"#, "comicId")?;

    let series = find_comic_series_node(&html).context("find ComicSeries JSON-LD node")?;
    let author_name = series
        .get("author")
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let genres = match series.get("genre") {
        Some(value) if value.is_array() => value
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect(),
        Some(value) if value.is_string() => vec![value.as_str().unwrap().to_string()],
        _ => Vec::new(),
    };
    let fallback_title = slug.clone();

    Ok(ExtractedComicDetail {
        slug: slug.clone(),
        comic_id,
        title: series
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&fallback_title)
            .to_string(),
        author: author_name,
        description: series.get("description").and_then(|v| v.as_str()).map(str::to_string),
        genres,
        thumbnail_url: series.get("image").and_then(|v| v.as_str()).map(str::to_string),
    })
}

pub fn find_comic_series_node(html: &str) -> Option<Value> {
    let re = Regex::new(r#"<script[^>]*type=["']application/ld\+json["'][^>]*>([\s\S]*?)</script>"#).unwrap();
    for capture in re.captures_iter(html) {
        let raw = capture.get(1)?.as_str().trim();
        let Ok(parsed): Result<Value, _> = serde_json::from_str(raw) else {
            continue;
        };
        let mut nodes = Vec::new();
        if let Some(graph) = parsed.get("@graph").and_then(|v| v.as_array()) {
            nodes.extend(graph.iter().cloned());
        }
        nodes.push(parsed);
        for node in nodes {
            let kind = node.get("@type");
            if kind.and_then(|v| v.as_str()) == Some("ComicSeries")
                || kind
                    .and_then(|v| v.as_array())
                    .map(|vals| vals.iter().any(|v| v.as_str() == Some("ComicSeries")))
                    .unwrap_or(false)
            {
                return Some(node);
            }
        }
    }
    None
}

pub fn extract_regex_group(source: &str, pattern: &str, name: &str) -> Result<String> {
    let re = Regex::new(pattern).context(format!("compile regex {name}"))?;
    let cap = re
        .captures(source)
        .and_then(|caps| caps.get(1))
        .context(format!("extract {name}"))?;
    Ok(cap.as_str().to_string())
}

pub async fn fetch_chapter_list(slug: &str, comic_id: &str) -> Result<Vec<ExtractedChapterStub>> {
    let url = format!(
        "{}/Comic/Services/ComicService.asmx/ChapterList?slug={slug}&comicId={comic_id}",
        nettruyen_url()
    );
    let response = reqwest::get(&url).await?;
    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch chapter list for {}: HTTP {}", slug, response.status());
    }
    let payload: Value = response.json().await?;
    let mut items = Vec::new();
    if let Some(data) = payload.get("data").and_then(|v| v.as_array()) {
        for item in data {
            let chapter_slug = item.get("chapter_slug").and_then(|v| v.as_str()).unwrap_or_default();
            let chapter_id = item.get("chapter_id").and_then(|v| v.as_str()).unwrap_or_default();
            let chapter_name = item.get("chapter_name").and_then(|v| v.as_str()).unwrap_or_default();
            let position = item.get("chapter_num").and_then(|v| v.as_i64()).unwrap_or_default() as i32;
            let origin_path_params = format!("{slug}/{chapter_slug}/{chapter_id}");
            let origin_url = format!("{}/truyen-tranh/{}", nettruyen_url(), origin_path_params);
            items.push(ExtractedChapterStub {
                chapter_id: chapter_id.to_string(),
                chapter_name: chapter_name.to_string(),
                position,
                origin_path_params: origin_path_params.clone(),
                origin_url,
            });
        }
    }
    Ok(items)
}

pub async fn fetch_chapter_image_candidates(origin_url: &str) -> Result<Vec<ImageCandidate>> {
    let response = reqwest::get(origin_url).await?;
    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch chapter page {}: HTTP {}", origin_url, response.status());
    }
    let html = response.text().await?;
    Ok(extract_chapter_image_candidates(&html))
}

pub fn extract_chapter_image_candidates(html: &str) -> Vec<ImageCandidate> {
    let tag_re = Regex::new(r#"<[^>]*\bdata-sv1=['"][^'"]+['"][^>]*>"#).unwrap();
    let sv1_re = Regex::new(r#"data-sv1=['\"]([^'\"]+)['\"]"#).unwrap();
    let sv2_re = Regex::new(r#"data-sv2=['\"]([^'\"]+)['\"]"#).unwrap();

    let mut candidates = Vec::new();
    for tag in tag_re.captures_iter(&html) {
        let text = tag.get(0).unwrap().as_str();
        let sv1 = sv1_re.captures(text).and_then(|cap| cap.get(1)).map(|m| m.as_str().to_string());
        let sv2 = sv2_re.captures(text).and_then(|cap| cap.get(1)).map(|m| m.as_str().to_string());
        if let Some(sv1) = sv1 {
            candidates.push(ImageCandidate { sv1, sv2 });
        }
    }
    candidates
}

pub async fn download_image(url: &str) -> Result<DownloadedImage> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("Origin", nettruyen_url())
        .header("Referer", nettruyen_url())
        .header("Accept", "*/*")
        .header("Content-Type", "application/octet-stream")
        .header("Access-Control-Allow-Origin", "*")
        .header("sec-fetch-mode", "cors")
        .header("sec-fetch-dest", "empty")
        .header("sec-fetch-site", "cross-site")
        .send()
        .await?;

    if !response.status().is_success() {
        anyhow::bail!("Failed to download image {}: HTTP {}", url, response.status());
    }

    let content_type: String = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("application/octet-stream")
        .to_string();
    let buffer = response.bytes().await?.to_vec();
    let extension = extension_from_url(url)
        .or_else(|| extension_from_content_type(&content_type))
        .unwrap_or_else(|| ".bin".to_string());

    Ok(DownloadedImage {
        buffer,
        content_type,
        extension,
    })
}

pub fn extension_from_url(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let path = parsed.path();
    let ext = path.rsplit('.').next()?;
    Some(format!(".{ext}"))
}

pub fn extension_from_content_type(content_type: &str) -> Option<String> {
    let subtype = content_type.split('/').nth(1)?.split(';').next()?;
    Some(format!(".{subtype}"))
}

pub async fn download_chapter_image(candidate: &ImageCandidate) -> Result<DownloadedImage> {
    match download_image(&candidate.sv1).await {
        Ok(image) => Ok(image),
        Err(err) => match &candidate.sv2 {
            Some(sv2) => download_image(sv2).await,
            None => Err(err),
        },
    }
}

pub async fn put_temp_object(file_name: &str, bytes: &[u8], _content_type: &str) -> Result<String> {
    let dir = image_storage_dir().join("temp");
    fs::create_dir_all(&dir).context("create temp dir")?;
    let target = dir.join(file_name);
    fs::write(&target, bytes).context("write temp object")?;
    let relative = format!("temp/{}", file_name);
    tracing::info!("image.local.temp.write path={} bytes={}", relative, bytes.len());
    Ok(relative)
}

pub async fn rename_to_permanent(temp_path: &str, permanent_path: &str) -> Result<()> {
    let root: std::path::PathBuf = image_storage_dir();
    let temp: std::path::PathBuf = root.join(temp_path);
    let permanent: std::path::PathBuf = root.join(permanent_path);
    if let Some(parent) = permanent.parent() {
        fs::create_dir_all(parent).context("create permanent dir")?;
    }
    if temp.exists() {
        fs::rename(&temp, &permanent).context("rename temp to permanent")?;
    }
    tracing::info!("image.local.promoted from={} to={}", temp_path, permanent_path);
    Ok(())
}

pub fn timestamp_tag() -> String {
    let now = chrono::Utc::now();
    format!(
        "{:04}{:02}{:02}_{:02}{:02}{:02}",
        now.year(),
        now.month(),
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
    )
}
