use std::env;
use std::path::PathBuf;

pub fn load_dotenv() {
    dotenvy::from_filename("../../../.env.bak").ok();
    dotenvy::from_filename(".env.bak.local").ok();
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
    let parsed =
        url::Url::parse(value).map_err(|_| format!("{name} must be a valid URL, got: {value}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!("{name} must use http:// or https://, got: {value}"));
    }
    Ok(value.to_string())
}

pub fn nettruyen_url() -> String {
    let raw: String =
        env::var("NETTRUYEN_URL").unwrap_or_else(|_| "https://nettruyenar.com".to_string());
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

    let _database_path = require_var("DATABASE_PATH")?;

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

    crate::db::Database::new()
        .initialize()
        .await
        .map_err(|error| format!("Unable to initialize SQLite database: {error}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn restore(name: &str, value: Option<String>) {
        unsafe {
            if let Some(value) = value {
                env::set_var(name, value);
            } else {
                env::remove_var(name);
            }
        }
    }

    #[tokio::test]
    async fn configuration_helpers_validate_values_and_preflight_database() {
        let _lock = crate::test_support::ENV_LOCK.lock().await;
        let keys = [
            "DATABASE_PATH",
            "CRAWL_API_KEY",
            "NETTRUYEN_URL",
            "PORT",
            "IMAGE_STORAGE_DIR",
        ];
        let previous: Vec<_> = keys.iter().map(|key| (*key, env::var(key).ok())).collect();

        unsafe {
            env::remove_var("DATABASE_PATH");
            env::remove_var("CRAWL_API_KEY");
            env::remove_var("NETTRUYEN_URL");
            env::remove_var("PORT");
            env::remove_var("IMAGE_STORAGE_DIR");
        }
        assert!(require_var("DATABASE_PATH").is_err());
        assert!(validate_http_url("URL", "ftp://example.test").is_err());
        assert_eq!(nettruyen_url(), "https://nettruyenar.com");
        assert_eq!(image_storage_dir(), PathBuf::from("./runtime"));

        unsafe {
            env::set_var("NETTRUYEN_URL", "invalid");
            env::set_var("IMAGE_STORAGE_DIR", "/tmp/warada-images");
        }
        assert_eq!(nettruyen_url(), "https://nettruyenar.com");
        assert_eq!(image_storage_dir(), PathBuf::from("/tmp/warada-images"));
        assert!(validate_http_url("URL", "https://example.test").is_ok());

        assert!(preflight_checks().await.is_err());
        let db_path = env::temp_dir().join(format!(
            "warada-config-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        unsafe {
            env::set_var("DATABASE_PATH", &db_path);
            env::set_var("CRAWL_API_KEY", "test-key");
            env::set_var("NETTRUYEN_URL", "https://example.test");
            env::set_var("PORT", "not-a-port");
        }
        assert!(preflight_checks().await.unwrap_err().contains("PORT"));
        unsafe {
            env::set_var("PORT", "3000");
        }
        preflight_checks().await.unwrap();
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(db_path.with_extension("db-wal"));
        let _ = std::fs::remove_file(db_path.with_extension("db-shm"));

        for (key, value) in previous {
            restore(key, value);
        }
    }
}
