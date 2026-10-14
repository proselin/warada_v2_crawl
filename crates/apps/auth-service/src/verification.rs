use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
};

use crate::{models::Identity, security::bearer_token, state::AppState};

pub(crate) async fn verify(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Identity>, StatusCode> {
    let authorization = headers
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let token = bearer_token(authorization).ok_or(StatusCode::UNAUTHORIZED)?;
    let claims = state
        .verification_cache
        .verify(token, &state.jwt_keys, &state.validation)?;
    Ok(Json(Identity {
        subject: claims.sub,
        principal_type: claims.principal_type,
        scopes: claims
            .scope
            .unwrap_or_default()
            .split_ascii_whitespace()
            .map(str::to_owned)
            .collect(),
    }))
}
