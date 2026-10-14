pub mod app;
pub mod broadcaster;
pub mod config;
pub mod db;
pub mod logging;
mod routes;
pub mod services;
pub mod state;

#[cfg(test)]
pub(crate) mod test_support {
    pub(crate) static ENV_LOCK: std::sync::LazyLock<tokio::sync::Mutex<()>> =
        std::sync::LazyLock::new(|| tokio::sync::Mutex::new(()));
}
