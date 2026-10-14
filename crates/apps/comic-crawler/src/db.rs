use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tokio_rusqlite::{
    Connection,
    rusqlite::{OptionalExtension, Row, params},
};
use tokio::sync::OnceCell;

#[derive(Clone, Debug)]
pub struct Database {
    path: Option<String>,
    connection: Arc<OnceCell<Connection>>,
}

#[derive(Clone, Debug)]
pub struct CrawlJob {
    pub id: i64,
    pub slug: String,
    pub job_type: String,
    pub status: String,
    pub error: Option<String>,
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

impl Database {
    pub fn new() -> Self {
        Self {
            path: env::var("DATABASE_PATH").ok(),
            connection: Arc::new(OnceCell::new()),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.path.is_some()
    }

    pub async fn initialize(&self) -> Result<(), String> {
        self.ensure_sqlite()
            .await
            .map(|_| ())
            .ok_or_else(|| "DATABASE_PATH is not configured".to_string())
    }

    async fn ensure_sqlite(&self) -> Option<Connection> {
        let path = self.path.clone()?;
        self.connection
            .get_or_try_init(|| async move {
                if let Some(parent) = Path::new(&path).parent()
                    && !parent.as_os_str().is_empty()
                {
                    std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
                }
                let connection = Connection::open(path)
                    .await
                    .map_err(|error| error.to_string())?;
                connection
                    .call(|connection| {
                        connection.busy_timeout(Duration::from_secs(10))?;
                        connection.pragma_update(None, "journal_mode", "WAL")?;
                        connection.pragma_update(None, "synchronous", "NORMAL")?;
                        connection.pragma_update(None, "foreign_keys", "ON")?;
                        connection.execute_batch(include_str!(
                            "../migrations/0001_init_database.sql"
                        ))
                    })
                    .await
                    .map_err(|error| error.to_string())?;
                Ok::<Connection, String>(connection)
            })
            .await
            .ok()
            .cloned()
    }

    pub async fn enqueue_crawl_job(
        &self,
        slug: &str,
        job_type: &str,
    ) -> Result<Option<i64>, String> {
        let connection = self
            .ensure_sqlite()
            .await
            .ok_or("SQLite is unavailable")?;
        let slug = slug.to_owned();
        let job_type = job_type.to_owned();
        connection
            .call(move |connection| {
                connection
                    .query_row(
                        "INSERT INTO crawl_jobs (slug, job_type) VALUES (?1, ?2)
                         ON CONFLICT (slug) WHERE status IN ('queued', 'running')
                         DO NOTHING RETURNING id",
                        params![slug, job_type],
                        |row| row.get(0),
                    )
                    .optional()
            })
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn claim_crawl_job(&self, worker_id: &str) -> Result<Option<CrawlJob>, String> {
        let connection = self
            .ensure_sqlite()
            .await
            .ok_or("SQLite is unavailable")?;
        let worker_id = worker_id.to_owned();
        connection
            .call(move |connection| {
                connection
                    .query_row(
                        "UPDATE crawl_jobs
                         SET status = 'running', worker_id = ?1, locked_at = CURRENT_TIMESTAMP,
                             attempts = attempts + 1
                         WHERE id = (
                             SELECT id FROM crawl_jobs WHERE status = 'queued'
                             ORDER BY created_at, id LIMIT 1
                         )
                         RETURNING id, slug, job_type, status, error",
                        [worker_id],
                        crawl_job,
                    )
                    .optional()
            })
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn has_crawl_job(&self, slug: &str) -> Result<bool, String> {
        let connection = self
            .ensure_sqlite()
            .await
            .ok_or("SQLite is unavailable")?;
        let slug = slug.to_owned();
        connection
            .call(move |connection| {
                connection
                    .query_row(
                        "SELECT 1 FROM crawl_jobs
                         WHERE slug = ?1 OR slug = (
                             SELECT origin_path_params FROM comics WHERE slug = ?1 LIMIT 1
                         ) LIMIT 1",
                        [slug],
                        |_| Ok(()),
                    )
                    .optional()
                    .map(|row| row.is_some())
            })
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn latest_crawl_job(&self, slug: &str) -> Result<Option<CrawlJob>, String> {
        let connection = self
            .ensure_sqlite()
            .await
            .ok_or("SQLite is unavailable")?;
        let slug = slug.to_owned();
        connection
            .call(move |connection| {
                connection
                    .query_row(
                        "SELECT id, slug, job_type, status, error FROM crawl_jobs
                         WHERE slug = ?1 OR slug = (
                             SELECT origin_path_params FROM comics WHERE slug = ?1 LIMIT 1
                         )
                         ORDER BY id DESC LIMIT 1",
                        [slug],
                        crawl_job,
                    )
                    .optional()
            })
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn finish_crawl_job(
        &self,
        job_id: i64,
        worker_id: &str,
        success: bool,
        error: Option<&str>,
    ) -> Result<(), String> {
        let connection = self
            .ensure_sqlite()
            .await
            .ok_or("SQLite is unavailable")?;
        let status = if success { "finished" } else { "failed" };
        let worker_id = worker_id.to_owned();
        let error = error.map(str::to_owned);
        let updated = connection
            .call(move |connection| {
                connection.execute(
                    "UPDATE crawl_jobs
                     SET status = ?1, error = ?2, finished_at = CURRENT_TIMESTAMP,
                         locked_at = NULL, worker_id = NULL
                     WHERE id = ?3 AND status = 'running' AND worker_id = ?4",
                    params![status, error, job_id, worker_id],
                )
            })
            .await
            .map_err(|error| error.to_string())?;
        if updated != 1 {
            return Err(format!(
                "crawl job {job_id} lock was lost before completion"
            ));
        }
        Ok(())
    }

    pub async fn next_id(&self, sequence: &str) -> Option<i64> {
        let table = match sequence {
            "comic_id_seq" => "comics",
            "chapter_id_seq" => "chapters",
            "image_id_seq" => "images",
            "tag_id_seq" => "tags",
            _ => return None,
        };
        let connection = self.ensure_sqlite().await?;
        let sequence = sequence.to_owned();
        let sql = format!(
            "INSERT INTO id_sequences (name, value)
             VALUES (?1, (SELECT COALESCE(MAX(id), 0) + 1 FROM {table}))
             ON CONFLICT(name) DO UPDATE SET value = max(id_sequences.value + 1, excluded.value)
             RETURNING value"
        );
        connection
            .call(move |connection| connection.query_row(&sql, [sequence], |row| row.get(0)))
            .await
            .ok()
    }

    pub async fn comic_exists_by_slug(&self, slug: &str) -> Option<bool> {
        let connection = self.ensure_sqlite().await?;
        let slug = slug.to_owned();
        connection
            .call(move |connection| {
                connection
                    .query_row(
                        "SELECT 1 FROM comics WHERE slug = ?1 LIMIT 1",
                        [slug],
                        |_| Ok(()),
                    )
                    .optional()
                    .map(|row| row.is_some())
            })
            .await
            .ok()
    }

    pub async fn get_comic_by_slug(&self, slug: &str) -> Option<crate::state::ComicRecord> {
        let connection = self.ensure_sqlite().await?;
        let slug = slug.to_owned();
        connection
            .call(move |connection| {
                connection
                    .query_row(
                        "SELECT id, slug, title, author, description, status, chapter_count,
                                crawling_status, origin_id, origin_url, origin_path_params,
                                thumb_image_id
                         FROM comics WHERE slug = ?1 LIMIT 1",
                        [slug],
                        comic_record,
                    )
                    .optional()
            })
            .await
            .ok()?
    }

    pub async fn list_comics_for_display(&self) -> Result<Vec<crate::state::ComicRecord>, String> {
        let connection = self
            .ensure_sqlite()
            .await
            .ok_or("SQLite is unavailable")?;
        connection
            .call(|connection| {
                let mut statement = connection.prepare(
                    "SELECT id, slug, title, author, description, status, chapter_count,
                            crawling_status, origin_id, origin_url, origin_path_params,
                            thumb_image_id
                     FROM comics ORDER BY title, slug",
                )?;
                statement
                    .query_map([], comic_record)?
                    .collect::<Result<Vec<_>, _>>()
            })
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn get_comic_for_display(
        &self,
        slug: &str,
    ) -> Result<Option<crate::state::ComicRecord>, String> {
        let connection = self
            .ensure_sqlite()
            .await
            .ok_or("SQLite is unavailable")?;
        let slug = slug.to_owned();
        connection
            .call(move |connection| {
                connection
                    .query_row(
                        "SELECT id, slug, title, author, description, status, chapter_count,
                                crawling_status, origin_id, origin_url, origin_path_params,
                                thumb_image_id
                         FROM comics WHERE slug = ?1 LIMIT 1",
                        [slug],
                        comic_record,
                    )
                    .optional()
            })
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn list_chapters_for_display(
        &self,
        comic_id: i64,
    ) -> Result<Vec<crate::state::ChapterRecord>, String> {
        let connection = self
            .ensure_sqlite()
            .await
            .ok_or("SQLite is unavailable")?;
        connection
            .call(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT id, comic_id, chapter_num, position, crawling_status,
                            origin_url, origin_path_params
                     FROM chapters WHERE comic_id = ?1 ORDER BY position, id",
                )?;
                statement
                    .query_map([comic_id], chapter_record)?
                    .collect::<Result<Vec<_>, _>>()
            })
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn list_crawl_jobs_for_display(&self) -> Result<Vec<CrawlJob>, String> {
        let connection = self
            .ensure_sqlite()
            .await
            .ok_or("SQLite is unavailable")?;
        connection
            .call(|connection| {
                let mut statement = connection.prepare(
                    "SELECT jobs.id, jobs.slug, jobs.job_type, jobs.status, jobs.error
                     FROM crawl_jobs AS jobs
                     JOIN (SELECT slug, MAX(id) AS id FROM crawl_jobs GROUP BY slug) AS latest
                       ON latest.id = jobs.id
                     ORDER BY jobs.id DESC LIMIT 50",
                )?;
                statement
                    .query_map([], crawl_job)?
                    .collect::<Result<Vec<_>, _>>()
            })
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn get_pending_chapters_for_comic(
        &self,
        comic_id: i64,
    ) -> Option<Vec<crate::state::ChapterRecord>> {
        let connection = self.ensure_sqlite().await?;
        connection
            .call(move |connection| {
                let mut statement = connection.prepare(
                    "SELECT id, comic_id, chapter_num, position, crawling_status,
                            origin_url, origin_path_params
                     FROM chapters
                     WHERE comic_id = ?1 AND crawling_status = '0000'
                     ORDER BY position, id",
                )?;
                statement
                    .query_map([comic_id], chapter_record)?
                    .collect::<Result<Vec<_>, _>>()
            })
            .await
            .ok()
    }

    pub async fn comic_exists_by_origin_path(&self, slug_n_id: &str) -> Option<bool> {
        let connection = self.ensure_sqlite().await?;
        let slug_n_id = slug_n_id.to_owned();
        connection
            .call(move |connection| {
                connection
                    .query_row(
                        "SELECT 1 FROM comics WHERE origin_path_params = ?1 LIMIT 1",
                        [slug_n_id],
                        |_| Ok(()),
                    )
                    .optional()
                    .map(|row| row.is_some())
            })
            .await
            .ok()
    }

    pub async fn insert_comic(&self, comic: &crate::state::ComicRecord) -> Option<()> {
        let connection = self.ensure_sqlite().await?;
        let comic = comic.clone();
        connection
            .call(move |connection| {
                connection.execute(
                    "INSERT INTO comics
                     (id, slug, title, author, description, status, chapter_count,
                      crawling_status, origin_id, origin_url, origin_path_params, thumb_image_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
                    params![
                        comic.id,
                        comic.slug,
                        comic.title,
                        comic.author,
                        comic.description,
                        comic.status,
                        comic.chapter_count,
                        comic.crawling_status,
                        comic.origin_id,
                        comic.origin_url,
                        comic.origin_path_params,
                        comic.thumb_image_id,
                    ],
                )?;
                Ok::<(), tokio_rusqlite::rusqlite::Error>(())
            })
            .await
            .ok()
    }

    pub async fn insert_chapter(&self, chapter: &crate::state::ChapterRecord) -> Option<()> {
        let connection = self.ensure_sqlite().await?;
        let chapter = chapter.clone();
        connection
            .call(move |connection| {
                connection.execute(
                    "INSERT INTO chapters
                     (id, comic_id, chapter_num, position, crawling_status,
                      origin_url, origin_path_params)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        chapter.id,
                        chapter.comic_id,
                        chapter.chapter_num,
                        chapter.position,
                        chapter.crawling_status,
                        chapter.origin_url,
                        chapter.origin_path_params,
                    ],
                )?;
                Ok::<(), tokio_rusqlite::rusqlite::Error>(())
            })
            .await
            .ok()
    }

    pub async fn insert_image(&self, image: &crate::state::ImageRecord) -> Option<()> {
        let connection = self.ensure_sqlite().await?;
        let image = image.clone();
        connection
            .call(move |connection| {
                connection.execute(
                    "INSERT INTO images
                     (id, chapter_id, position, file_name, file_path, origin_url, type)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        image.id,
                        image.chapter_id,
                        image.position,
                        image.file_name,
                        image.file_path,
                        image.origin_url,
                        image.image_type,
                    ],
                )?;
                Ok::<(), tokio_rusqlite::rusqlite::Error>(())
            })
            .await
            .ok()
    }

    pub async fn update_chapter_status(
        &self,
        chapter_id: i64,
        crawling_status: &str,
    ) -> Option<()> {
        let connection = self.ensure_sqlite().await?;
        let crawling_status = crawling_status.to_owned();
        connection
            .call(move |connection| {
                connection.execute(
                    "UPDATE chapters SET crawling_status = ?1 WHERE id = ?2",
                    params![crawling_status, chapter_id],
                )?;
                Ok::<(), tokio_rusqlite::rusqlite::Error>(())
            })
            .await
            .ok()
    }

    pub async fn resolve_or_create_tags(&self, names: &[String]) -> Option<Vec<i64>> {
        let connection = self.ensure_sqlite().await?;
        let mut seen = HashMap::new();
        for name in names {
            let normalized = normalize_tag_name(name);
            if !normalized.is_empty() {
                seen.entry(normalized).or_insert_with(|| name.trim().to_owned());
            }
        }
        if seen.is_empty() {
            return Some(Vec::new());
        }

        connection
            .call(move |connection| {
                seen.into_iter()
                    .map(|(normalized, display_name)| {
                        connection.query_row(
                            "INSERT INTO tags (id, name, normalized_name, comic_count)
                             VALUES (
                                 (SELECT COALESCE(MAX(id), 0) + 1 FROM tags),
                                 ?1, ?2, 0
                             )
                             ON CONFLICT(normalized_name) DO UPDATE
                             SET normalized_name = excluded.normalized_name
                             RETURNING id",
                            params![display_name, normalized],
                            |row| row.get(0),
                        )
                    })
                    .collect()
            })
            .await
            .ok()
    }

    pub async fn increment_comic_count(&self, tag_ids: &[i64]) -> Option<()> {
        let connection = self.ensure_sqlite().await?;
        let tag_ids = tag_ids.to_vec();
        connection
            .call(move |connection| {
                for tag_id in tag_ids {
                    connection.execute(
                        "UPDATE tags SET comic_count = comic_count + 1 WHERE id = ?1",
                        [tag_id],
                    )?;
                }
                Ok::<(), tokio_rusqlite::rusqlite::Error>(())
            })
            .await
            .ok()
    }

    pub async fn link_comic_tag(&self, comic_id: i64, tag_id: i64) -> Option<()> {
        let connection = self.ensure_sqlite().await?;
        connection
            .call(move |connection| {
                connection.execute(
                    "INSERT INTO comic_tags (comic_id, tag_id) VALUES (?1, ?2)",
                    params![comic_id, tag_id],
                )?;
                Ok::<(), tokio_rusqlite::rusqlite::Error>(())
            })
            .await
            .ok()
    }
}

fn normalize_tag_name(name: &str) -> String {
    name.trim().to_lowercase()
}

fn crawl_job(row: &Row<'_>) -> tokio_rusqlite::rusqlite::Result<CrawlJob> {
    Ok(CrawlJob {
        id: row.get("id")?,
        slug: row.get("slug")?,
        job_type: row.get("job_type")?,
        status: row.get("status")?,
        error: row.get("error")?,
    })
}

fn comic_record(row: &Row<'_>) -> tokio_rusqlite::rusqlite::Result<crate::state::ComicRecord> {
    Ok(crate::state::ComicRecord {
        id: row.get("id")?,
        slug: row.get("slug")?,
        title: row.get("title")?,
        author: row.get("author")?,
        description: row.get("description")?,
        status: row.get("status")?,
        chapter_count: row.get("chapter_count")?,
        crawling_status: row.get("crawling_status")?,
        origin_id: row.get("origin_id")?,
        origin_url: row.get("origin_url")?,
        origin_path_params: row.get("origin_path_params")?,
        thumb_image_id: row.get("thumb_image_id")?,
    })
}

fn chapter_record(row: &Row<'_>) -> tokio_rusqlite::rusqlite::Result<crate::state::ChapterRecord> {
    Ok(crate::state::ChapterRecord {
        id: row.get("id")?,
        comic_id: row.get("comic_id")?,
        chapter_num: row.get("chapter_num")?,
        position: row.get("position")?,
        crawling_status: row.get("crawling_status")?,
        origin_url: row.get("origin_url")?,
        origin_path_params: row.get("origin_path_params")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    async fn sqlite_queue_deduplicates_and_assigns_a_job_to_one_connection() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let path = env::temp_dir().join(format!("warada-{}-{unique}.db", std::process::id()));
        let path = path.to_string_lossy().into_owned();
        let database = || Database {
            path: Some(path.clone()),
            connection: Arc::new(OnceCell::new()),
        };
        let first = database();
        let second = database();
        first.initialize().await.expect("initialize test database");

        let slug = format!("queue-test-{}-{unique}", std::process::id());
        let job_id = first
            .enqueue_crawl_job(&slug, "comic")
            .await
            .expect("enqueue first request")
            .expect("first request creates job");
        assert_eq!(
            second.enqueue_crawl_job(&slug, "comic").await.unwrap(),
            None
        );

        let (first_claim, second_claim) = tokio::join!(
            first.claim_crawl_job("test-worker-a"),
            second.claim_crawl_job("test-worker-b"),
        );
        let first_claim = first_claim.expect("first worker claims");
        let second_claim = second_claim.expect("second worker claims");
        assert_ne!(first_claim.is_some(), second_claim.is_some());

        let (claimed, worker, worker_id) = match (first_claim, second_claim) {
            (Some(job), None) => (job, &first, "test-worker-a"),
            (None, Some(job)) => (job, &second, "test-worker-b"),
            _ => panic!("exactly one worker must claim the job"),
        };
        assert_eq!(claimed.id, job_id);
        worker
            .finish_crawl_job(job_id, worker_id, true, None)
            .await
            .expect("finish claimed job");
        assert_eq!(
            first.latest_crawl_job(&slug).await.unwrap().unwrap().status,
            "finished"
        );

        drop(first);
        drop(second);
        tokio::time::sleep(Duration::from_millis(20)).await;
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{path}{suffix}"));
        }
    }
}
