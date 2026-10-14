use std::{path::Path, sync::Arc};

use jsonwebtoken::{Algorithm, Validation};

use crate::{
    database::DatabaseService,
    jwt_cache::JwtVerificationCache,
    jwt_keys::JwtKeys,
    security::{hash_password, hash_secret},
    state::AppState,
};

pub(crate) async fn app_state() -> Arc<AppState> {
    let db = DatabaseService::connect(":memory:")
        .await
        .expect("connect test database");
    let pepper: Arc<[u8]> = Arc::from(b"test-pepper-with-at-least-32-bytes".as_slice());
    let password_hash = hash_password("correct horse battery staple".to_owned(), pepper.clone())
        .await
        .expect("hash test password");
    db.create_user("user-1", "user@example.test", &password_hash, 1)
        .await
        .expect("create test user");
    let secret_hash = hash_secret("test-service-secret".to_owned())
        .await
        .expect("hash service secret");
    db.create_service_client("service", &secret_hash, "api:read api:write", 1)
        .await
        .expect("create service client");
    db.create_public_client(
        "public",
        "https://app.example/callback",
        "api:read api:write",
        1,
    )
    .await
    .expect("create public client");

    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let jwt_keys = JwtKeys::load(
        &fixture_root.join("jwt-keys"),
        &fixture_root.join("jwt-private/new.pem"),
        "new".to_owned(),
        "old".to_owned(),
    )
    .expect("load test JWT keys");
    let mut validation = Validation::new(Algorithm::RS256);
    validation.leeway = 0;
    validation.set_issuer(&["https://issuer.test/"]);
    validation.set_audience(&["test-api"]);

    Arc::new(AppState {
        db,
        admin_key: Arc::from(b"test-admin-key-with-more-than-32-bytes".as_slice()),
        password_pepper: pepper,
        jwt_keys: Arc::new(jwt_keys),
        validation: Arc::new(validation),
        verification_cache: Arc::new(JwtVerificationCache::new(16)),
        issuer: Arc::from("https://issuer.test/"),
        audience: Arc::from("test-api"),
        token_ttl: 300,
        allow_implicit: false,
        cookie_secure: false,
    })
}
