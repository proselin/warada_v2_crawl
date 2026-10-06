use std::env;
use std::path::PathBuf;

pub fn load_dotenv() {
    dotenvy::from_filename(".env").ok();
    dotenvy::from_filename(".env.local").ok();
}

fn require_var(name: &str) -> Result<String, String> {
    let value: String = env::var(name)
        .map_err(|_| format!("{name} is required; set it before starting the app"))?;
    let trimmed: &str = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{name} cannot be empty"));
    }
    Ok(trimmed.to_string())
}

fn validate_http_url(name: &str, value: &str) -> Result<String, String> {
    let parsed = url::Url::parse(value)
        .map_err(|_| format!("{name} must be a valid URL, got: {value}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!("{name} must use http:// or https://, got: {value}"));
    }
    Ok(value.to_string())
}

fn validate_postgres_url(value: &str) -> Result<String, String> {
    let parsed = url::Url::parse(value)
        .map_err(|_| format!("DATABASE_URL must be a valid postgres:// or postgresql:// URL, got: {value}"))?;
    if !matches!(parsed.scheme(), "postgres" | "postgresql") {
        return Err(format!("DATABASE_URL must use postgres:// or postgresql://, got: {value}"));
    }
    Ok(value.to_string())
}

pub fn nettruyen_url() -> String {
    let raw: String = env::var("NETTRUYEN_URL").unwrap_or_else(|_| "https://nettruyenar.com".to_string());
    if let Ok(value) = validate_http_url("NETTRUYEN_URL", &raw) {
        value
    } else {
        "https://nettruyenar.com".to_string()
    }
}

pub fn image_storage_dir() -> PathBuf {
    env::var_os("IMAGE_STORAGE_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./runtime"))
}

pub async fn preflight_checks() -> Result<(), String> {
    load_dotenv();

    let database_url: String = require_var("DATABASE_URL")?;
    let _ = validate_postgres_url(&database_url)?;

    let api_key: String = require_var("CRAWL_API_KEY")?;
    if api_key.trim().is_empty() {
        return Err("CRAWL_API_KEY cannot be empty".to_string());
    }

    let nettruyen: String = require_var("NETTRUYEN_URL")?;
    let _ = validate_http_url("NETTRUYEN_URL", &nettruyen)?;

    let port: String = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let port: u16 = port
        .trim()
        .parse()
        .map_err(|_| format!("PORT must be a valid TCP port, got: {port}"))?;
    let _ = port;

    let (client, connection) = tokio_postgres::connect(&database_url, tokio_postgres::NoTls)
        .await
        .map_err(|err| format!("Unable to connect to PostgreSQL at DATABASE_URL: {err}"))?;
    let queue_table = client
        .query_one("SELECT to_regclass('public.crawl_jobs')::text", &[])
        .await
        .map_err(|err| format!("Unable to inspect crawl queue schema: {err}"))?
        .get::<_, Option<String>>(0);
    if queue_table.is_none() {
        return Err("Database is not initialized; run migrations/0001_init_database.sql".to_string());
    }
    tokio::spawn(async move {
        if let Err(err) = connection.await {
            tracing::warn!(error = %err, "postgres_connection_task_exited");
        }
    });
    drop(client);

    Ok(())
}
