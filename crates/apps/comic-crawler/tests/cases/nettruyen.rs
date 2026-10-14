use comic_crawler::services::nettruyen::{
    extension_from_content_type, extension_from_url, extract_chapter_image_candidates,
    extract_regex_group, find_comic_series_node,
};

#[test]
fn nettruyen_parsers_extract_expected_values() {
    let html = r#"<script>gOpts.comicSlug = "one-piece";</script>"#;
    assert_eq!(
        extract_regex_group(
            html,
            r#"gOpts\.comicSlug\s*=\s*["']([^"']+)["']"#,
            "comicSlug"
        )
        .unwrap(),
        "one-piece"
    );

    let html = r#"
        <script type="application/ld+json">
            {
                "@graph": [
                    { "@type": "WebSite", "name": "Example" },
                    { "@type": "ComicSeries", "name": "One Piece", "genre": ["Action", "Adventure"] }
                ]
            }
        </script>
    "#;
    let node = find_comic_series_node(html).unwrap();
    assert_eq!(node.get("name").and_then(|v| v.as_str()), Some("One Piece"));
    assert_eq!(
        node.get("genre").and_then(|v| v.as_array()).unwrap().len(),
        2
    );

    let html = r#"
        <img data-sv1="https://img-a.example/1.jpg" data-sv2="https://img-b.example/1.jpg" />
        <img data-sv1="https://img-a.example/2.jpg" />
    "#;
    let candidates = extract_chapter_image_candidates(html);
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].sv1, "https://img-a.example/1.jpg");
    assert_eq!(
        candidates[0].sv2.as_deref(),
        Some("https://img-b.example/1.jpg")
    );
    assert_eq!(candidates[1].sv2, None);

    assert_eq!(
        extension_from_url("https://example.com/images/cover.jpg"),
        Some(".jpg".to_string())
    );
    assert_eq!(
        extension_from_content_type("image/jpeg"),
        Some(".jpeg".to_string())
    );
}
