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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chapter_number: Option<String>,
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
