CREATE TABLE IF NOT EXISTS comics (
    id INTEGER PRIMARY KEY,
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
    thumb_image_id INTEGER
);

CREATE TABLE IF NOT EXISTS chapters (
    id INTEGER PRIMARY KEY,
    comic_id INTEGER NOT NULL REFERENCES comics(id) ON DELETE CASCADE,
    chapter_num TEXT NOT NULL,
    position INTEGER NOT NULL,
    crawling_status TEXT NOT NULL,
    origin_url TEXT NOT NULL,
    origin_path_params TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS images (
    id INTEGER PRIMARY KEY,
    chapter_id INTEGER REFERENCES chapters(id) ON DELETE CASCADE,
    position INTEGER,
    file_name TEXT NOT NULL,
    file_path TEXT NOT NULL,
    origin_url TEXT,
    type TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS tags (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    normalized_name TEXT NOT NULL UNIQUE,
    comic_count INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS comic_tags (
    comic_id INTEGER NOT NULL REFERENCES comics(id) ON DELETE CASCADE,
    tag_id INTEGER NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (comic_id, tag_id)
);

CREATE TABLE IF NOT EXISTS id_sequences (
    name TEXT PRIMARY KEY,
    value INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS crawl_jobs (
    id INTEGER PRIMARY KEY,
    slug TEXT NOT NULL,
    job_type TEXT NOT NULL CHECK (job_type IN ('comic', 'retry')),
    status TEXT NOT NULL DEFAULT 'queued'
        CHECK (status IN ('queued', 'running', 'finished', 'failed')),
    attempts INTEGER NOT NULL DEFAULT 0,
    error TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    locked_at TEXT,
    worker_id TEXT,
    finished_at TEXT
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
