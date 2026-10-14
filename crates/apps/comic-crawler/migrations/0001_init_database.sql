BEGIN;

CREATE SEQUENCE IF NOT EXISTS comic_id_seq;
CREATE SEQUENCE IF NOT EXISTS chapter_id_seq;
CREATE SEQUENCE IF NOT EXISTS image_id_seq;
CREATE SEQUENCE IF NOT EXISTS tag_id_seq;

CREATE TABLE IF NOT EXISTS comics (
    id BIGINT PRIMARY KEY DEFAULT nextval('comic_id_seq'),
    slug TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    author TEXT,
    description TEXT,
    status TEXT NOT NULL,
    chapter_count INTEGER,
    crawling_status TEXT NOT NULL,
    origin_id TEXT,
    origin_url TEXT,
    origin_path_params TEXT UNIQUE,
    thumb_image_id BIGINT
);

CREATE TABLE IF NOT EXISTS chapters (
    id BIGINT PRIMARY KEY DEFAULT nextval('chapter_id_seq'),
    comic_id BIGINT NOT NULL REFERENCES comics(id) ON DELETE CASCADE,
    chapter_num TEXT NOT NULL,
    position INTEGER NOT NULL,
    crawling_status TEXT NOT NULL,
    origin_url TEXT NOT NULL,
    origin_path_params TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS images (
    id BIGINT PRIMARY KEY DEFAULT nextval('image_id_seq'),
    chapter_id BIGINT REFERENCES chapters(id) ON DELETE CASCADE,
    position INTEGER,
    file_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    origin_url TEXT,
    type TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tags (
    id BIGINT PRIMARY KEY DEFAULT nextval('tag_id_seq'),
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL UNIQUE,
    comic_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS comic_tags (
    comic_id BIGINT NOT NULL REFERENCES comics(id) ON DELETE CASCADE,
    tag_id BIGINT NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (comic_id, tag_id)
);

CREATE TABLE IF NOT EXISTS crawl_jobs (
    id BIGSERIAL PRIMARY KEY,
    slug TEXT NOT NULL,
    job_type TEXT NOT NULL CHECK (job_type IN ('comic', 'retry')),
    status TEXT NOT NULL DEFAULT 'queued' CHECK (status IN ('queued', 'running', 'finished', 'failed')),
    attempts INTEGER NOT NULL DEFAULT 0,
    error TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    locked_at TIMESTAMPTZ,
    worker_id TEXT,
    finished_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS chapters_comic_position_idx
    ON chapters (comic_id, position, id);

CREATE INDEX IF NOT EXISTS chapters_pending_idx
    ON chapters (comic_id, crawling_status);

CREATE INDEX IF NOT EXISTS images_chapter_position_idx
    ON images (chapter_id, position);

CREATE INDEX IF NOT EXISTS crawl_jobs_claim_idx
    ON crawl_jobs (status, created_at, id);

CREATE UNIQUE INDEX IF NOT EXISTS crawl_jobs_active_slug_idx
    ON crawl_jobs (slug)
    WHERE status IN ('queued', 'running');

COMMIT;