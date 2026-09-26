use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrawlProgressEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub message: String,
    pub data: CrawlProgressData,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CrawlProgressData {
    pub chapter_id: i64,
    pub chapter_number: String,
    pub status: Option<String>,
    pub comic_slug: String,
}

#[derive(Clone, Debug, Default)]
pub struct Broadcaster {
    listeners: Arc<RwLock<HashMap<String, Vec<mpsc::UnboundedSender<CrawlProgressEvent>>>>>,
}

impl Broadcaster {
    pub fn subscribe(&self, comic_slug: &str) -> mpsc::UnboundedReceiver<CrawlProgressEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        let mut listeners = self.listeners.write().unwrap();
        listeners.entry(comic_slug.to_string()).or_default().push(tx);
        rx
    }

    pub fn broadcast(&self, comic_slug: &str, event: &CrawlProgressEvent) {
        let listeners = self.listeners.read().unwrap();
        if let Some(items) = listeners.get(comic_slug) {
            for item in items {
                let _ = item.send(event.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broadcaster_delivers_only_to_matching_comic_slug() {
        let broadcaster = Broadcaster::default();
        let mut first = broadcaster.subscribe("one-piece");
        let mut second = broadcaster.subscribe("one-piece");
        let mut other = broadcaster.subscribe("other");

        let event = CrawlProgressEvent {
            event_type: "processing".to_string(),
            message: "Chapter 1 started".to_string(),
            data: CrawlProgressData {
                chapter_id: 42,
                chapter_number: "1".to_string(),
                status: None,
                comic_slug: "one-piece".to_string(),
            },
        };

        broadcaster.broadcast("one-piece", &event);

        assert_eq!(first.try_recv().unwrap().message, "Chapter 1 started");
        assert_eq!(second.try_recv().unwrap().data.chapter_id, 42);
        assert!(other.try_recv().is_err());
    }
}
