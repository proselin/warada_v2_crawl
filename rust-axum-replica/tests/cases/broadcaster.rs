use axum_replica::broadcaster::{Broadcaster, CrawlProgressData, CrawlProgressEvent};

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
