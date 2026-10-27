mod config;
mod database;
mod jwt_cache;
mod jwt_keys;
mod management;
mod models;
mod oauth;
mod security;
mod state;
mod tokens;
mod verification;

use std::{error::Error, sync::Arc};

use axum::{
    Router,
    routing::{get, patch, post},
};
use jsonwebtoken::{Algorithm, Validation};
use state::AppState;
use tracing::{error, info};

use crate::{
    config::Config, database::DatabaseService, jwt_cache::JwtVerificationCache, jwt_keys::JwtKeys,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    info!("starting auth service");

    // Load and validate all configuration up front so startup failures are
    // reported with the exact offending variable.
    let config = match Config::load() {
        Ok(config) => config,
        Err(error) => {
            error!(%error, "configuration error; refusing to start");
            return Err(error.into());
        }
    };

    info!(
        public_keys_dir = %config.public_keys_dir.display(),
        private_key_path = %config.private_key_path.display(),
        "loading JWT keys"
    );
    let jwt_keys = match JwtKeys::load(
        &config.public_keys_dir,
        &config.private_key_path,
        config.signing_key_id.clone(),
        config.legacy_key_id.clone(),
    ) {
        Ok(keys) => {
            info!("JWT keys loaded");
            Arc::new(keys)
        }
        Err(error) => {
            error!(%error, "failed to load JWT keys; refusing to start");
            return Err(error);
        }
    };

    let mut validation = Validation::new(Algorithm::RS256);
    validation.leeway = 0;
    validation.set_issuer(&[config.issuer.as_str()]);
    validation.set_audience(&[config.audience.as_str()]);

    info!(database_path = %config.database_path, "connecting to database");
    let db = match DatabaseService::connect(&config.database_path).await {
        Ok(db) => {
            info!("database connected and schema ready");
            db
        }
        Err(error) => {
            error!(%error, "failed to connect to database; refusing to start");
            return Err(std::io::Error::other(error).into());
        }
    };

    let state = Arc::new(AppState {
        db,
        admin_key: Arc::from(config.admin_key.into_bytes()),
        password_pepper: Arc::from(config.password_pepper.into_bytes()),
        jwt_keys,
        validation: Arc::new(validation),
        verification_cache: Arc::new(JwtVerificationCache::new(config.cache_capacity)),
        issuer: Arc::from(config.issuer),
        audience: Arc::from(config.audience),
        token_ttl: config.token_ttl,
        allow_implicit: config.allow_implicit,
        cookie_secure: config.cookie_secure,
    });
    let addr = config.addr;

    let app = Router::new()
        .route("/health", get(health))
        .route("/v1/verify", get(verification::verify))
        .route(
            "/v1/users",
            get(management::list_users).post(management::create_user),
        )
        .route(
            "/v1/users/{id}",
            get(management::get_user)
                .patch(management::update_user)
                .delete(management::deactivate_user),
        )
        .route(
            "/v1/service-clients",
            get(management::list_clients).post(management::create_client),
        )
        .route(
            "/v1/service-clients/{client_id}",
            patch(management::update_client).delete(management::deactivate_client),
        )
        .route("/v1/public-clients", post(oauth::create_public_client))
        .route(
            "/oauth/authorize",
            get(oauth::authorize_page).post(oauth::authorize),
        )
        .route("/oauth/token", post(oauth::token))
        .with_state(state);

    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(error) => {
            error!(%addr, %error, "failed to bind listen address; refusing to start");
            return Err(error.into());
        }
    };
    info!(%addr, "auth service listening");
    if let Err(error) = axum::serve(listener, app).await {
        error!(%error, "server terminated with error");
        return Err(error.into());
    }
    info!("auth service shut down");
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}

#[cfg(test)]
mod test_support;

#[cfg(test)]
#[path = "../tests/auth.rs"]
mod tests;
