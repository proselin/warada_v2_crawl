use axum::http::StatusCode;
use jsonwebtoken::{Algorithm, Header, encode};

use crate::{
    jwt_keys::JwtKeys,
    models::{Claims, PrincipalType, TokenResponse},
    security::unix_now,
};

pub(crate) fn issue_token(
    keys: &JwtKeys,
    issuer: &str,
    audience: &str,
    token_ttl: u64,
    subject: String,
    principal_type: PrincipalType,
    scopes: Vec<String>,
) -> Result<TokenResponse, StatusCode> {
    let now = unix_now()? as u64;
    let claims = Claims {
        sub: subject,
        iss: issuer.to_owned(),
        aud: audience.to_owned(),
        exp: now + token_ttl,
        iat: now,
        principal_type,
        scope: Some(scopes.join(" ")),
    };
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(keys.active_key_id().to_owned());
    let access_token = encode(&header, &claims, keys.signing_key())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(TokenResponse {
        access_token,
        token_type: "Bearer",
        expires_in: token_ttl,
    })
}
