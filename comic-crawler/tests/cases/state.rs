use axum_replica::state::{AppState, ChapterRecord, ComicRecord, normalize_tag_name};

#[tokio::test]
async fn state_tracks_tags_comics_chapters_and_active_slugs() {
    let state = AppState::default();
    assert_eq!(normalize_tag_name(" Action "), "action");
    assert_eq!(normalize_tag_name("Fantasy"), "fantasy");

    let ids = state
        .resolve_or_create_tags(&[
            "Action".to_string(),
            " action ".to_string(),
            "Adventure".to_string(),
            "adventure".to_string(),
        ])
        .await;
    assert_eq!(ids.len(), 2);
    assert_eq!(state.shared.tags.read().unwrap().len(), 2);
    state.increment_comic_count(&ids).await;
    assert!(
        state
            .shared
            .tags
            .read()
            .unwrap()
            .values()
            .all(|tag| tag.comic_count == 1)
    );

    assert_eq!(state.next_id("comic_id_seq").await, 1);
    assert_eq!(state.next_id("comic_id_seq").await, 2);
    assert_eq!(state.next_id("tag_id_seq").await, 3);
    assert!(!state.is_active("one-piece"));
    state.mark_active("one-piece");
    assert!(state.is_active("one-piece"));
    state.mark_inactive("one-piece");
    assert!(!state.is_active("one-piece"));

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
    state.insert_comic(comic.clone()).await;
    assert!(state.comic_exists_by_slug("one-piece").await);
    assert!(state.comic_exists_by_origin_path("one-piece-123").await);

    let chapter = ChapterRecord {
        id: 20,
        comic_id: comic.id,
        chapter_num: "1".to_string(),
        position: 1,
        crawling_status: "0000".to_string(),
        origin_url: "https://example.test/ch/1".to_string(),
        origin_path_params: "one-piece/1/20".to_string(),
    };
    state.insert_chapter(chapter.clone()).await;
    state.update_chapter_status(chapter.id, "9999").await;
    let chapters = state.get_chapters_for_comic(comic.id);
    assert_eq!(chapters.len(), 1);
    assert_eq!(chapters[0].chapter_num, "1");
    assert_eq!(chapters[0].crawling_status, "9999");
}
