use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, RwLock};

use crate::broadcaster::Broadcaster;

#[derive(Clone, Debug, Default)]
pub struct AppState {
    pub shared: Arc<AppStateInner>,
}

#[derive(Debug, Default)]
pub struct AppStateInner {
    pub active_slugs: Mutex<HashSet<String>>,
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
    pub fn next_id(&self, sequence: &str) -> i64 {
        let mut seq = self.shared.sequences.lock().unwrap();
        let next = seq.entry(sequence.to_string()).or_insert(1);
        let value = *next;
        *next += 1;
        value
    }

    pub fn is_active(&self, slug: &str) -> bool {
        self.shared.active_slugs.lock().unwrap().contains(slug)
    }

    pub fn mark_active(&self, slug: &str) {
        self.shared.active_slugs.lock().unwrap().insert(slug.to_string());
    }

    pub fn mark_inactive(&self, slug: &str) {
        self.shared.active_slugs.lock().unwrap().remove(slug);
    }

    pub fn comic_exists_by_slug(&self, slug: &str) -> bool {
        self.shared.comics.read().unwrap().contains_key(slug)
    }

    pub fn comic_exists_by_origin_path(&self, slug_n_id: &str) -> bool {
        self.shared
            .comics
            .read()
            .unwrap()
            .values()
            .any(|comic| comic.origin_path_params.as_deref() == Some(slug_n_id))
    }

    pub fn insert_comic(&self, comic: ComicRecord) {
        self.shared.comics.write().unwrap().insert(comic.slug.clone(), comic);
    }

    pub fn insert_chapter(&self, chapter: ChapterRecord) {
        self.shared.chapters.write().unwrap().insert(chapter.id, chapter);
    }

    pub fn insert_image(&self, image: ImageRecord) {
        self.shared.images.write().unwrap().insert(image.id, image);
    }

    pub fn update_chapter_status(&self, chapter_id: i64, crawling_status: &str) {
        let mut chapters = self.shared.chapters.write().unwrap();
        if let Some(chapter) = chapters.get_mut(&chapter_id) {
            chapter.crawling_status = crawling_status.to_string();
        }
    }

    pub fn resolve_or_create_tags(&self, names: &[String]) -> Vec<i64> {
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

            let tag_id = self.next_id("tag_id_seq");
            let record = TagRecord {
                id: tag_id,
                name: display_name,
                normalized_name: normalized.clone(),
                comic_count: 0,
            };
            self.shared.tags.write().unwrap().insert(normalized.clone(), record);
            ids.push(tag_id);
        }
        ids
    }

    pub fn increment_comic_count(&self, tag_ids: &[i64]) {
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

    pub fn link_comic_tag(&self, comic_id: i64, tag_id: i64) {
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

pub fn normalize_tag_name(name: &str) -> String {
    name.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_tag_name_is_case_and_space_insensitive() {
        assert_eq!(normalize_tag_name(" Action "), "action");
        assert_eq!(normalize_tag_name("Fantasy"), "fantasy");
    }

    #[test]
    fn resolve_or_create_tags_deduplicates_and_tracks_counts() {
        let state = AppState::default();
        let ids = state.resolve_or_create_tags(&[
            "Action".to_string(),
            " action ".to_string(),
            "Adventure".to_string(),
            "adventure".to_string(),
        ]);

        assert_eq!(ids.len(), 2);
        assert_eq!(state.shared.tags.read().unwrap().len(), 2);

        state.increment_comic_count(&ids);
        let tags = state.shared.tags.read().unwrap();
        let values: Vec<i32> = tags.values().map(|tag| tag.comic_count).collect();
        assert_eq!(values, vec![1, 1]);
    }

    #[test]
    fn next_id_and_active_tracking_are_sequence_safe() {
        let state = AppState::default();

        assert_eq!(state.next_id("comic_id_seq"), 1);
        assert_eq!(state.next_id("comic_id_seq"), 2);
        assert_eq!(state.next_id("tag_id_seq"), 1);

        assert!(!state.is_active("one-piece"));
        state.mark_active("one-piece");
        assert!(state.is_active("one-piece"));
        state.mark_inactive("one-piece");
        assert!(!state.is_active("one-piece"));
    }

    #[test]
    fn comic_and_chapter_records_are_inserted_and_found_by_filters() {
        let state = AppState::default();
        let comic = ComicRecord {
            id: 10,
            slug: "one-piece".to_string(),
            title: "One Piece".to_string(),
            author: Some("Eiichiro Oda".to_string()),
            description: None,
            status: "OnGoing".to_string(),
            chapter_count: Some(3),
            crawling_status: "9999".to_string(),
            origin_id: Some("123".to_string()),
            origin_url: Some("https://example.test/truyen-tranh/one-piece".to_string()),
            origin_path_params: Some("one-piece-123".to_string()),
            thumb_image_id: None,
        };
        state.insert_comic(comic.clone());
        assert!(state.comic_exists_by_slug("one-piece"));
        assert!(state.comic_exists_by_origin_path("one-piece-123"));

        let chapter = ChapterRecord {
            id: 20,
            comic_id: comic.id,
            chapter_num: "1".to_string(),
            position: 1,
            crawling_status: "0000".to_string(),
            origin_url: "https://example.test/ch/1".to_string(),
            origin_path_params: "one-piece/1/20".to_string(),
        };
        state.insert_chapter(chapter.clone());
        state.update_chapter_status(chapter.id, "9999");

        let chapters = state.get_chapters_for_comic(comic.id);
        assert_eq!(chapters.len(), 1);
        assert_eq!(chapters[0].chapter_num, "1");
        assert_eq!(chapters[0].crawling_status, "9999");
    }
}
