use std::sync::Arc;

use jsonwebtoken::Validation;

use crate::{database::DatabaseService, jwt_cache::JwtVerificationCache, jwt_keys::JwtKeys};

pub(crate) struct AppState {
    pub(crate) db: DatabaseService,
    pub(crate) admin_key: Arc<[u8]>,
    pub(crate) password_pepper: Arc<[u8]>,
    pub(crate) jwt_keys: Arc<JwtKeys>,
    pub(crate) validation: Arc<Validation>,
    pub(crate) verification_cache: Arc<JwtVerificationCache>,
    pub(crate) issuer: Arc<str>,
    pub(crate) audience: Arc<str>,
    pub(crate) token_ttl: u64,
    pub(crate) allow_implicit: bool,
    pub(crate) cookie_secure: bool,
}
