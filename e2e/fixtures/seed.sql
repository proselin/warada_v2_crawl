INSERT INTO comics (
  id, slug, title, author, description, status, chapter_count,
  crawling_status, origin_id, origin_url, origin_path_params, thumb_image_id
) VALUES (
  900001, 'e2e-library', 'E2E Test Comic', 'Warada', 'Seeded E2E catalog item',
  'ongoing', 1, '0000', 'e2e', 'https://example.test/e2e-library',
  'e2e-library', NULL
);

INSERT INTO chapters (
  id, comic_id, chapter_num, position, crawling_status, origin_url, origin_path_params
) VALUES (
  900001, 900001, '1', 1, '0000', 'https://example.test/e2e-library/chapter-1',
  'e2e-library/chapter-1'
);

INSERT INTO crawl_jobs (slug, job_type, status)
VALUES ('e2e-library', 'comic', 'finished');
