use comic_crawler::broadcaster::{Broadcaster, CrawlProgressData, CrawlProgressEvent};

#[test]
fn broadcaster_only_delivers_to_matching_comic_slug() {
    let broadcaster = Broadcaster::default();
    let mut first = broadcaster.subscribe("one-piece");
    let mut second = broadcaster.subscribe("one-piece");
    let mut other = broadcaster.subscribe("other");
    let event = CrawlProgressEvent {
        event_type: "processing".to_string(),
        message: "Chapter 1 started".to_string(),
        data: CrawlProgressData {
            chapter_id: Some(42),
            chapter_number: Some("1".to_string()),
            status: None,
            comic_slug: "one-piece".to_string(),
        },
    };
    broadcaster.broadcast("one-piece", &event);
    assert_eq!(first.try_recv().unwrap().message, "Chapter 1 started");
    assert_eq!(second.try_recv().unwrap().data.chapter_id, Some(42));
    assert!(other.try_recv().is_err());
}

#[test]
fn job_progress_event_does_not_require_chapter_fields() {
    let event = CrawlProgressEvent {
        event_type: "finished".to_string(),
        message: "Crawl job finished".to_string(),
        data: CrawlProgressData {
            chapter_id: None,
            chapter_number: None,
            status: Some("finished".to_string()),
            comic_slug: "one-piece".to_string(),
        },
    };

    let payload = serde_json::to_value(event).unwrap();
    assert_eq!(payload["type"], "finished");
    assert_eq!(payload["data"]["status"], "finished");
    assert!(payload["data"].get("chapter_id").is_none());
    assert!(payload["data"].get("chapter_number").is_none());
}
