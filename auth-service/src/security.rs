use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use argon2::{Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version};
use axum::http::{HeaderMap, StatusCode, header::AUTHORIZATION};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use getrandom::fill;
use subtle::ConstantTimeEq;

pub(crate) enum PasswordCheck {
    Invalid,
    Valid,
    ValidNeedsRehash,
}

pub(crate) fn require_admin(headers: &HeaderMap, admin_key: &[u8]) -> Result<(), StatusCode> {
    let provided = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(bearer_token)
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if bool::from(provided.as_bytes().ct_eq(admin_key)) {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

pub(crate) fn bearer_token(value: &str) -> Option<&str> {
    let mut parts = value.split_ascii_whitespace();
    if !parts.next()?.eq_ignore_ascii_case("bearer") {
        return None;
    }
    let token = parts.next()?;
    parts.next().is_none().then_some(token)
}

pub(crate) fn normalize_email(email: String) -> Result<String, StatusCode> {
    let email = email.trim().to_ascii_lowercase();
    let Some((local, domain)) = email.split_once('@') else {
        return Err(StatusCode::BAD_REQUEST);
    };
    if email.len() > 254
        || local.is_empty()
        || domain.is_empty()
        || domain.contains('@')
        || email.chars().any(char::is_whitespace)
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(email)
}

pub(crate) fn validate_password(password: &str) -> Result<(), StatusCode> {
    if !(12..=1024).contains(&password.len()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

pub(crate) fn normalize_scopes(scopes: Vec<String>) -> Result<Vec<String>, StatusCode> {
    if scopes.is_empty()
        || scopes.len() > 32
        || scopes.iter().any(|scope| {
            scope.is_empty()
                || scope.len() > 100
                || !scope.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
        })
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let mut scopes = scopes;
    scopes.sort();
    scopes.dedup();
    Ok(scopes)
}

pub(crate) fn split_scopes(scopes: &str) -> Vec<String> {
    scopes.split_ascii_whitespace().map(str::to_owned).collect()
}

pub(crate) async fn hash_secret(secret: String) -> Result<String, StatusCode> {
    tokio::task::spawn_blocking(move || {
        Argon2::default()
            .hash_password(secret.as_bytes())
            .map(|hash| hash.to_string())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

pub(crate) async fn hash_password(
    password: String,
    pepper: Arc<[u8]>,
) -> Result<String, StatusCode> {
    tokio::task::spawn_blocking(move || {
        Argon2::new_with_secret(
            &pepper,
            Algorithm::default(),
            Version::default(),
            Params::default(),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .hash_password(password.as_bytes())
        .map(|hash| hash.to_string())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

pub(crate) async fn verify_password(
    password: String,
    stored_hash: String,
    pepper: Arc<[u8]>,
) -> Result<PasswordCheck, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let Ok(hash) = PasswordHash::new(&stored_hash) else {
            return Ok(PasswordCheck::Invalid);
        };
        let peppered = Argon2::new_with_secret(
            &pepper,
            Algorithm::default(),
            Version::default(),
            Params::default(),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if peppered.verify_password(password.as_bytes(), &hash).is_ok() {
            return Ok(PasswordCheck::Valid);
        }
        if Argon2::default()
            .verify_password(password.as_bytes(), &hash)
            .is_ok()
        {
            Ok(PasswordCheck::ValidNeedsRehash)
        } else {
            Ok(PasswordCheck::Invalid)
        }
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    .and_then(|result| result)
}

pub(crate) async fn verify_secret(secret: String, hash: String) -> Result<bool, StatusCode> {
    tokio::task::spawn_blocking(move || {
        let Ok(hash) = PasswordHash::new(&hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(secret.as_bytes(), &hash)
            .is_ok()
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) fn random_token(bytes: usize) -> Result<String, StatusCode> {
    let mut value = vec![0; bytes];
    fill(&mut value).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(URL_SAFE_NO_PAD.encode(value))
}

pub(crate) fn unix_now() -> Result<i64, StatusCode> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub(crate) fn is_unique_violation(error: &tokio_postgres::Error) -> bool {
    error
        .code()
        .is_some_and(|code| code == &tokio_postgres::error::SqlState::UNIQUE_VIOLATION)
}
