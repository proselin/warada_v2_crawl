use std::collections::HashMap;
use std::env;
use std::sync::{Arc, RwLock};

use tokio_postgres::{Client, NoTls};

#[derive(Clone, Debug)]
pub struct Database {
    url: Option<String>,
    backend: Arc<RwLock<DatabaseBackend>>,
}

#[derive(Clone, Debug)]
enum DatabaseBackend {
    Memory,
    Postgres(Arc<Client>),
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
            url: env::var("DATABASE_URL").ok(),
            backend: Arc::new(RwLock::new(DatabaseBackend::Memory)),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.url.is_some()
    }

    pub async fn enqueue_crawl_job(&self, slug: &str, job_type: &str) -> Result<Option<i64>, String> {
        let client = self.ensure_postgres().await.ok_or("PostgreSQL is unavailable")?;
        let row = client
            .query_opt(
                "INSERT INTO crawl_jobs (slug, job_type) VALUES ($1, $2) ON CONFLICT (slug) WHERE status IN ('queued', 'running') DO NOTHING RETURNING id",
                &[&slug, &job_type],
            )
            .await
            .map_err(|err| err.to_string())?;
        Ok(row.map(|row| row.get(0)))
    }

    pub async fn claim_crawl_job(&self, worker_id: &str) -> Result<Option<CrawlJob>, String> {
        let client = self.ensure_postgres().await.ok_or("PostgreSQL is unavailable")?;
        let row = client
            .query_opt(
                "UPDATE crawl_jobs SET status = 'running', worker_id = $1, locked_at = now(), attempts = attempts + 1 WHERE id = (SELECT id FROM crawl_jobs WHERE status = 'queued' ORDER BY created_at, id FOR UPDATE SKIP LOCKED LIMIT 1) RETURNING id, slug, job_type, status, error",
                &[&worker_id],
            )
            .await
            .map_err(|err| err.to_string())?;
        Ok(row.map(|row| CrawlJob {
            id: row.get("id"),
            slug: row.get("slug"),
            job_type: row.get("job_type"),
            status: row.get("status"),
            error: row.get("error"),
        }))
    }

    pub async fn has_crawl_job(&self, slug: &str) -> Result<bool, String> {
        let client = self.ensure_postgres().await.ok_or("PostgreSQL is unavailable")?;
        client
            .query_opt(
                "SELECT 1 FROM crawl_jobs WHERE slug = $1 OR slug = (SELECT origin_path_params FROM comics WHERE slug = $1 LIMIT 1) LIMIT 1",
                &[&slug],
            )
            .await
            .map(|row| row.is_some())
            .map_err(|err| err.to_string())
    }

    pub async fn latest_crawl_job(&self, slug: &str) -> Result<Option<CrawlJob>, String> {
        let client = self.ensure_postgres().await.ok_or("PostgreSQL is unavailable")?;
        let row = client
            .query_opt(
                "SELECT id, slug, job_type, status, error FROM crawl_jobs WHERE slug = $1 OR slug = (SELECT origin_path_params FROM comics WHERE slug = $1 LIMIT 1) ORDER BY id DESC LIMIT 1",
                &[&slug],
            )
            .await
            .map_err(|err| err.to_string())?;
        Ok(row.map(|row| CrawlJob {
            id: row.get("id"),
            slug: row.get("slug"),
            job_type: row.get("job_type"),
            status: row.get("status"),
            error: row.get("error"),
        }))
    }

    pub async fn finish_crawl_job(
        &self,
        job_id: i64,
        worker_id: &str,
        success: bool,
        error: Option<&str>,
    ) -> Result<(), String> {
        let client = self.ensure_postgres().await.ok_or("PostgreSQL is unavailable")?;
        let status = if success { "finished" } else { "failed" };
        let updated = client
            .execute(
                "UPDATE crawl_jobs SET status = $1, error = $2, finished_at = now(), locked_at = NULL, worker_id = NULL WHERE id = $3 AND status = 'running' AND worker_id = $4",
                &[&status, &error, &job_id, &worker_id],
            )
            .await
            .map_err(|err| err.to_string())?;
        if updated != 1 {
            return Err(format!("crawl job {job_id} lock was lost before completion"));
        }
        Ok(())
    }

    async fn ensure_postgres(&self) -> Option<Arc<Client>> {
        {
            let backend = self.backend.read().unwrap();
            if let DatabaseBackend::Postgres(client) = &*backend {
                return Some(client.clone());
            }
        }

        let url = self.url.clone()?;
        let (client, connection) = tokio_postgres::connect(&url, NoTls).await.ok()?;
        tokio::spawn(async move {
            if let Err(err) = connection.await {
                tracing::warn!(error = %err, "postgres_connection_task_exited");
            }
        });

        let client = Arc::new(client);
        let mut backend = self.backend.write().unwrap();
        *backend = DatabaseBackend::Postgres(client.clone());
        Some(client)
    }

    pub async fn next_id(&self, sequence: &str) -> Option<i64> {
        let client = self.ensure_postgres().await?;
        let row = client
            .query_one("SELECT nextval($1::regclass) AS id", &[&sequence])
            .await
            .ok()?;
        Some(row.get::<_, i64>(0))
    }

    pub async fn comic_exists_by_slug(&self, slug: &str) -> Option<bool> {
        let client = self.ensure_postgres().await?;
        let row = client
            .query_opt("SELECT 1 FROM comics WHERE slug = $1 LIMIT 1", &[&slug])
            .await
            .ok()?;
        Some(row.is_some())
    }

    pub async fn get_comic_by_slug(&self, slug: &str) -> Option<crate::state::ComicRecord> {
        let client = self.ensure_postgres().await?;
        let row = client
            .query_opt(
                "SELECT id, slug, title, author, description, status, chapter_count, crawling_status, origin_id, origin_url, origin_path_params, thumb_image_id FROM comics WHERE slug = $1 LIMIT 1",
                &[&slug],
            )
            .await
            .ok()??;
        Some(crate::state::ComicRecord {
            id: row.get("id"),
            slug: row.get("slug"),
            title: row.get("title"),
            author: row.get("author"),
            description: row.get("description"),
            status: row.get("status"),
            chapter_count: row.get("chapter_count"),
            crawling_status: row.get("crawling_status"),
            origin_id: row.get("origin_id"),
            origin_url: row.get("origin_url"),
            origin_path_params: row.get("origin_path_params"),
            thumb_image_id: row.get("thumb_image_id"),
        })
    }

    pub async fn get_pending_chapters_for_comic(&self, comic_id: i64) -> Option<Vec<crate::state::ChapterRecord>> {
        let client = self.ensure_postgres().await?;
        let rows = client
            .query(
                "SELECT id, comic_id, chapter_num, position, crawling_status, origin_url, origin_path_params FROM chapters WHERE comic_id = $1 AND crawling_status = '0000' ORDER BY position, id",
                &[&comic_id],
            )
            .await
            .ok()?;
        Some(rows.into_iter().map(|row| crate::state::ChapterRecord {
            id: row.get("id"),
            comic_id: row.get("comic_id"),
            chapter_num: row.get("chapter_num"),
            position: row.get("position"),
            crawling_status: row.get("crawling_status"),
            origin_url: row.get("origin_url"),
            origin_path_params: row.get("origin_path_params"),
        }).collect())
    }

    pub async fn comic_exists_by_origin_path(&self, slug_n_id: &str) -> Option<bool> {
        let client = self.ensure_postgres().await?;
        let row = client
            .query_opt(
                "SELECT 1 FROM comics WHERE origin_path_params = $1 LIMIT 1",
                &[&slug_n_id],
            )
            .await
            .ok()?;
        Some(row.is_some())
    }

    pub async fn insert_comic(&self, comic: &crate::state::ComicRecord) -> Option<()> {
        let client = self.ensure_postgres().await?;
        client
            .execute(
                "INSERT INTO comics (id, slug, title, author, description, status, chapter_count, crawling_status, origin_id, origin_url, origin_path_params, thumb_image_id) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
                &[
                    &comic.id,
                    &comic.slug,
                    &comic.title,
                    &comic.author,
                    &comic.description,
                    &comic.status,
                    &comic.chapter_count,
                    &comic.crawling_status,
                    &comic.origin_id,
                    &comic.origin_url,
                    &comic.origin_path_params,
                    &comic.thumb_image_id,
                ],
            )
            .await
            .ok()?;
        Some(())
    }

    pub async fn insert_chapter(&self, chapter: &crate::state::ChapterRecord) -> Option<()> {
        let client = self.ensure_postgres().await?;
        client
            .execute(
                "INSERT INTO chapters (id, comic_id, chapter_num, position, crawling_status, origin_url, origin_path_params) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[
                    &chapter.id,
                    &chapter.comic_id,
                    &chapter.chapter_num,
                    &chapter.position,
                    &chapter.crawling_status,
                    &chapter.origin_url,
                    &chapter.origin_path_params,
                ],
            )
            .await
            .ok()?;
        Some(())
    }

    pub async fn insert_image(&self, image: &crate::state::ImageRecord) -> Option<()> {
        let client = self.ensure_postgres().await?;
        client
            .execute(
                "INSERT INTO images (id, chapter_id, position, file_name, file_path, origin_url, type) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[
                    &image.id,
                    &image.chapter_id,
                    &image.position,
                    &image.file_name,
                    &image.file_path,
                    &image.origin_url,
                    &image.image_type,
                ],
            )
            .await
            .ok()?;
        Some(())
    }

    pub async fn update_chapter_status(&self, chapter_id: i64, crawling_status: &str) -> Option<()> {
        let client = self.ensure_postgres().await?;
        client
            .execute(
                "UPDATE chapters SET crawling_status = $1 WHERE id = $2",
                &[&crawling_status, &chapter_id],
            )
            .await
            .ok()?;
        Some(())
    }

    pub async fn resolve_or_create_tags(&self, names: &[String]) -> Option<Vec<i64>> {
        let client = self.ensure_postgres().await?;

        let mut seen: HashMap<String, String> = HashMap::new();
        for name in names {
            let normalized = normalize_tag_name(name);
            if !normalized.is_empty() && !seen.contains_key(&normalized) {
                seen.insert(normalized, name.trim().to_string());
            }
        }
        if seen.is_empty() {
            return Some(Vec::new());
        }

        let normalized_names: Vec<&str> = seen.keys().map(|value| value.as_str()).collect();
        let rows = client
            .query(
                "SELECT id, normalized_name FROM tags WHERE normalized_name = ANY($1)",
                &[&normalized_names],
            )
            .await
            .ok()?;
        let mut existing = HashMap::new();
        for row in rows {
            let id: i64 = row.get("id");
            let normalized_name: String = row.get("normalized_name");
            existing.insert(normalized_name, id);
        }

        let mut ids = Vec::new();
        for (normalized, display_name) in seen {
            if let Some(id) = existing.get(&normalized) {
                ids.push(*id);
                continue;
            }

            let id = next_id_with_client(client.as_ref(), "tag_id_seq").await?;
            client
                .execute(
                    "INSERT INTO tags (id, name, normalized_name, comic_count) VALUES ($1, $2, $3, $4)",
                    &[&id, &display_name, &normalized, &0i32],
                )
                .await
                .ok()?;
            ids.push(id);
        }
        Some(ids)
    }

    pub async fn increment_comic_count(&self, tag_ids: &[i64]) -> Option<()> {
        if tag_ids.is_empty() {
            return Some(());
        }
        let client = self.ensure_postgres().await?;
        client
            .execute(
                "UPDATE tags SET comic_count = comic_count + 1 WHERE id = ANY($1)",
                &[&tag_ids.to_vec()],
            )
            .await
            .ok()?;
        Some(())
    }

    pub async fn link_comic_tag(&self, comic_id: i64, tag_id: i64) -> Option<()> {
        let client = self.ensure_postgres().await?;
        client
            .execute(
                "INSERT INTO comic_tags (comic_id, tag_id) VALUES ($1, $2)",
                &[&comic_id, &tag_id],
            )
            .await
            .ok()?;
        Some(())
    }
}

fn normalize_tag_name(name: &str) -> String {
    name.trim().to_lowercase()
}

async fn next_id_with_client(client: &Client, sequence: &str) -> Option<i64> {
    let row = client
        .query_one("SELECT nextval($1::regclass) AS id", &[&sequence])
        .await
        .ok()?;
    Some(row.get(0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[tokio::test]
    #[ignore = "requires a dedicated PostgreSQL database with migrations applied"]
    async fn postgres_queue_deduplicates_and_assigns_a_job_to_one_connection() {
        let url = env::var("CRAWLER_TEST_DATABASE_URL")
            .expect("set CRAWLER_TEST_DATABASE_URL to a dedicated migrated database");

        let database = || Database {
            url: Some(url.clone()),
            backend: Arc::new(RwLock::new(DatabaseBackend::Memory)),
        };
        let first = database();
        let second = database();
        let client = first.ensure_postgres().await.expect("connect test database");
        let queued: i64 = client
            .query_one("SELECT count(*) FROM crawl_jobs WHERE status = 'queued'", &[])
            .await
            .expect("query queue; apply migrations/0001_init_database.sql")
            .get(0);
        assert_eq!(queued, 0, "use a dedicated test database with no pending jobs");

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let slug = format!("queue-test-{}-{unique}", std::process::id());
        let job_id = first
            .enqueue_crawl_job(&slug, "comic")
            .await
            .expect("enqueue first request")
            .expect("first request creates job");
        assert_eq!(second.enqueue_crawl_job(&slug, "comic").await.unwrap(), None);

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
        assert_eq!(first.latest_crawl_job(&slug).await.unwrap().unwrap().status, "finished");
        client
            .execute("DELETE FROM crawl_jobs WHERE id = $1", &[&job_id])
            .await
            .expect("remove test job");
    }
}
