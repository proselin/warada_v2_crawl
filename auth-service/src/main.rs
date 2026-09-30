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

use std::{env, error::Error, net::SocketAddr, path::Path, sync::Arc};

use axum::{
    Router,
    routing::{get, patch, post},
};
use jsonwebtoken::{Algorithm, Validation};
use state::AppState;
use tracing::info;

use crate::{database::DatabaseService, jwt_cache::JwtVerificationCache, jwt_keys::JwtKeys};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let signing_key_id = env::var("JWT_SIGNING_KEY_ID")?;
    let legacy_key_id = env::var("JWT_LEGACY_KEY_ID").unwrap_or_else(|_| signing_key_id.clone());
    let private_key_path = env::var("JWT_PRIVATE_KEY_PATH")?;
    let jwt_keys = Arc::new(JwtKeys::load(
        Path::new(&env::var("JWT_PUBLIC_KEYS_DIR")?),
        Path::new(&private_key_path),
        signing_key_id,
        legacy_key_id,
    )?);
    let issuer = env::var("JWT_ISSUER")?;
    let audience = env::var("JWT_AUDIENCE")?;
    let admin_key = env::var("AUTH_ADMIN_API_KEY")?;
    let password_pepper = env::var("AUTH_PASSWORD_PEPPER")?;
    let token_ttl = env::var("ACCESS_TOKEN_TTL_SECONDS")
        .unwrap_or_else(|_| "900".to_owned())
        .parse::<u64>()?;
    let cache_capacity = env::var("JWT_VERIFY_CACHE_CAPACITY")
        .unwrap_or_else(|_| "10000".to_owned())
        .parse::<usize>()?;
    if !(60..=3600).contains(&token_ttl)
        || admin_key.len() < 32
        || password_pepper.len() < 32
        || !(1..=1_000_000).contains(&cache_capacity)
    {
        return Err("token TTL must be 60-3600 seconds, admin key and password pepper at least 32 bytes, and JWT cache capacity 1-1000000".into());
    }

    let mut validation = Validation::new(Algorithm::RS256);
    validation.leeway = 0;
    validation.set_issuer(&[issuer.as_str()]);
    validation.set_audience(&[audience.as_str()]);
    let state = Arc::new(AppState {
        db: DatabaseService::connect(&env::var("DATABASE_URL")?).await?,
        admin_key: Arc::from(admin_key.into_bytes()),
        password_pepper: Arc::from(password_pepper.into_bytes()),
        jwt_keys,
        validation: Arc::new(validation),
        verification_cache: Arc::new(JwtVerificationCache::new(cache_capacity)),
        issuer: Arc::from(issuer),
        audience: Arc::from(audience),
        token_ttl,
        allow_implicit: env::var("ENABLE_IMPLICIT_FLOW")
            .is_ok_and(|value| value.eq_ignore_ascii_case("true")),
        cookie_secure: !env::var("AUTH_COOKIE_SECURE")
            .is_ok_and(|value| value.eq_ignore_ascii_case("false")),
    });
    let addr = env::var("AUTH_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8081".to_owned())
        .parse::<SocketAddr>()?;

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

    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "auth service listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health() -> &'static str {
    "ok"
}
