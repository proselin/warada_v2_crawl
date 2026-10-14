use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use crate::broadcaster::Broadcaster;
use crate::db::Database;

#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub shared: Arc<AppStateInner>,
}

#[derive(Debug, Default)]
pub struct AppStateInner {
    pub active_slugs: Mutex<HashSet<String>>,
    pub database: Database,
    pub comics: RwLock<HashMap<String, ComicRecord>>,
    pub chapters: RwLock<HashMap<i64, ChapterRecord>>,
    pub images: RwLock<HashMap<i64, ImageRecord>>,
    pub tags: RwLock<HashMap<String, TagRecord>>,
    pub comic_tags: RwLock<HashMap<i64, HashSet<i64>>>,
    pub sequences: Mutex<HashMap<String, i64>>,
    pub broadcaster: Broadcaster,
}

#[derive(Clone, Debug)]
pub struct ComicRecord {
    pub id: i64,
    pub slug: String,
    pub title: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub status: String,
    pub chapter_count: Option<i32>,
    pub crawling_status: String,
    pub origin_id: Option<String>,
    pub origin_url: Option<String>,
    pub origin_path_params: Option<String>,
    pub thumb_image_id: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct ChapterRecord {
    pub id: i64,
    pub comic_id: i64,
    pub chapter_num: String,
    pub position: i32,
    pub crawling_status: String,
    pub origin_url: String,
    pub origin_path_params: String,
}

#[derive(Clone, Debug)]
pub struct ImageRecord {
    pub id: i64,
    pub chapter_id: Option<i64>,
    pub position: Option<i32>,
    pub file_name: String,
    pub file_path: String,
    pub origin_url: Option<String>,
    pub image_type: String,
}

#[derive(Clone, Debug)]
pub struct TagRecord {
    pub id: i64,
    pub name: String,
    pub normalized_name: String,
    pub comic_count: i32,
}

impl AppState {
    pub async fn next_id(&self, sequence: &str) -> i64 {
        if self.shared.database.is_enabled() {
            if let Some(id) =
                time_database_operation("next_id", self.shared.database.next_id(sequence)).await
            {
                return id;
            }
        }
        let mut seq = self.shared.sequences.lock().unwrap();
        let next = seq.entry(sequence.to_string()).or_insert(1);
        let value = *next;
        *next += 1;
        value
    }

    pub fn is_active(&self, slug: &str) -> bool {
        self.shared.active_slugs.lock().unwrap().contains(slug)
    }

    pub fn try_mark_active(&self, slug: &str) -> bool {
        self.shared
            .active_slugs
            .lock()
            .unwrap()
            .insert(slug.to_string())
    }

    pub fn mark_active(&self, slug: &str) {
        self.shared
            .active_slugs
            .lock()
            .unwrap()
            .insert(slug.to_string());
    }

    pub fn mark_inactive(&self, slug: &str) {
        self.shared.active_slugs.lock().unwrap().remove(slug);
    }

    pub async fn comic_exists_by_slug(&self, slug: &str) -> bool {
        if self.shared.database.is_enabled() {
            if let Some(exists) = time_database_operation(
                "comic_exists_by_slug",
                self.shared.database.comic_exists_by_slug(slug),
            )
            .await
            {
                return exists;
            }
        }
        self.shared.comics.read().unwrap().contains_key(slug)
    }

    pub async fn comic_exists_by_origin_path(&self, slug_n_id: &str) -> bool {
        if self.shared.database.is_enabled() {
            if let Some(exists) = time_database_operation(
                "comic_exists_by_origin_path",
                self.shared.database.comic_exists_by_origin_path(slug_n_id),
            )
            .await
            {
                return exists;
            }
        }
        self.shared
            .comics
            .read()
            .unwrap()
            .values()
            .any(|comic| comic.origin_path_params.as_deref() == Some(slug_n_id))
    }

    pub async fn insert_comic(&self, comic: ComicRecord) {
        if self.shared.database.is_enabled() {
            let _ =
                time_database_operation("insert_comic", self.shared.database.insert_comic(&comic))
                    .await;
        }
        self.shared
            .comics
            .write()
            .unwrap()
            .insert(comic.slug.clone(), comic);
    }

    pub async fn insert_chapter(&self, chapter: ChapterRecord) {
        if self.shared.database.is_enabled() {
            let _ = time_database_operation(
                "insert_chapter",
                self.shared.database.insert_chapter(&chapter),
            )
            .await;
        }
        self.shared
            .chapters
            .write()
            .unwrap()
            .insert(chapter.id, chapter);
    }

    pub async fn insert_image(&self, image: ImageRecord) {
        if self.shared.database.is_enabled() {
            let _ =
                time_database_operation("insert_image", self.shared.database.insert_image(&image))
                    .await;
        }
        self.shared.images.write().unwrap().insert(image.id, image);
    }

    pub async fn update_chapter_status(&self, chapter_id: i64, crawling_status: &str) {
        if self.shared.database.is_enabled() {
            let _ = time_database_operation(
                "update_chapter_status",
                self.shared
                    .database
                    .update_chapter_status(chapter_id, crawling_status),
            )
            .await;
        }
        let mut chapters = self.shared.chapters.write().unwrap();
        if let Some(chapter) = chapters.get_mut(&chapter_id) {
            chapter.crawling_status = crawling_status.to_string();
        }
    }

    pub async fn resolve_or_create_tags(&self, names: &[String]) -> Vec<i64> {
        if self.shared.database.is_enabled() {
            if let Some(ids) = time_database_operation(
                "resolve_or_create_tags",
                self.shared.database.resolve_or_create_tags(names),
            )
            .await
            {
                return ids;
            }
        }

        let mut by_normalized: HashMap<String, String> = HashMap::new();
        for name in names {
            let normalized = normalize_tag_name(name);
            if normalized.is_empty() || by_normalized.contains_key(&normalized) {
                continue;
            }
            by_normalized.insert(normalized, name.trim().to_string());
        }

        let mut ids = Vec::new();
        for (normalized, display_name) in by_normalized {
            let existing = {
                let tags = self.shared.tags.read().unwrap();
                tags.get(&normalized).cloned()
            };

            if let Some(existing) = existing {
                ids.push(existing.id);
                continue;
            }

            let tag_id = self.next_id("tag_id_seq").await;
            let record = TagRecord {
                id: tag_id,
                name: display_name,
                normalized_name: normalized.clone(),
                comic_count: 0,
            };
            self.shared
                .tags
                .write()
                .unwrap()
                .insert(normalized.clone(), record);
            ids.push(tag_id);
        }
        ids
    }

    pub async fn increment_comic_count(&self, tag_ids: &[i64]) {
        if self.shared.database.is_enabled() {
            let _ = time_database_operation(
                "increment_comic_count",
                self.shared.database.increment_comic_count(tag_ids),
            )
            .await;
        }
        if tag_ids.is_empty() {
            return;
        }
        let mut tags = self.shared.tags.write().unwrap();
        for tag_id in tag_ids {
            if let Some(tag) = tags.values_mut().find(|tag| tag.id == *tag_id) {
                tag.comic_count += 1;
            }
        }
    }

    pub async fn link_comic_tag(&self, comic_id: i64, tag_id: i64) {
        if self.shared.database.is_enabled() {
            let _ = time_database_operation(
                "link_comic_tag",
                self.shared.database.link_comic_tag(comic_id, tag_id),
            )
            .await;
        }
        let mut map = self.shared.comic_tags.write().unwrap();
        map.entry(comic_id).or_default().insert(tag_id);
    }

    pub fn get_chapters_for_comic(&self, comic_id: i64) -> Vec<ChapterRecord> {
        self.shared
            .chapters
            .read()
            .unwrap()
            .values()
            .filter(|chapter| chapter.comic_id == comic_id)
            .cloned()
            .collect()
    }
}

async fn time_database_operation<T>(
    operation: &'static str,
    action: impl Future<Output = Option<T>>,
) -> Option<T> {
    let started_at = Instant::now();
    let result = action.await;
    tracing::info!(
        target: "performance",
        operation = %operation,
        request_id = crate::logging::REQUEST_ID.try_with(|id| *id).ok(),
        success = result.is_some(),
        duration_ms = started_at.elapsed().as_millis() as u64,
        "database operation completed"
    );
    result
}

pub fn normalize_tag_name(name: &str) -> String {
    name.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_store_fallbacks_keep_catalog_and_tags_consistent() {
        let _env_lock = crate::test_support::ENV_LOCK.lock().await;
        let state = AppState::default();
        assert_eq!(state.next_id("other").await, 1);
        assert_eq!(state.next_id("other").await, 2);
        assert!(!state.comic_exists_by_slug("missing").await);
        assert!(!state.comic_exists_by_origin_path("missing").await);

        let comic = ComicRecord {
            id: 10,
            slug: "memory-comic".to_owned(),
            title: "Memory Comic".to_owned(),
            author: None,
            description: None,
            status: "ongoing".to_owned(),
            chapter_count: None,
            crawling_status: "queued".to_owned(),
            origin_id: None,
            origin_url: None,
            origin_path_params: Some("memory-comic-10".to_owned()),
            thumb_image_id: None,
        };
        state.insert_comic(comic.clone()).await;
        assert!(state.comic_exists_by_slug("memory-comic").await);
        assert!(state.comic_exists_by_origin_path("memory-comic-10").await);

        let chapter = ChapterRecord {
            id: 11,
            comic_id: comic.id,
            chapter_num: "1".to_owned(),
            position: 1,
            crawling_status: "pending".to_owned(),
            origin_url: "https://example.test/chapter".to_owned(),
            origin_path_params: "memory-comic/1".to_owned(),
        };
        state.insert_chapter(chapter.clone()).await;
        state
            .insert_image(ImageRecord {
                id: 12,
                chapter_id: Some(chapter.id),
                position: Some(1),
                file_name: "page.jpg".to_owned(),
                file_path: "comic/page.jpg".to_owned(),
                origin_url: None,
                image_type: "chapter".to_owned(),
            })
            .await;
        state.update_chapter_status(999, "ignored").await;
        state.update_chapter_status(chapter.id, "finished").await;
        assert_eq!(
            state.get_chapters_for_comic(comic.id)[0].crawling_status,
            "finished"
        );

        let ids = state
            .resolve_or_create_tags(&[" Action ".to_owned(), "action".to_owned(), "   ".to_owned()])
            .await;
        assert_eq!(ids.len(), 1);
        assert_eq!(
            state.resolve_or_create_tags(&["ACTION".to_owned()]).await,
            ids
        );
        state.increment_comic_count(&[]).await;
        state.increment_comic_count(&ids).await;
        assert_eq!(state.shared.tags.read().unwrap()["action"].comic_count, 1);
        state.link_comic_tag(comic.id, ids[0]).await;
        state.link_comic_tag(comic.id, ids[0]).await;
        assert_eq!(state.shared.comic_tags.read().unwrap()[&comic.id].len(), 1);
    }
}
