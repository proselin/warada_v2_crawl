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
