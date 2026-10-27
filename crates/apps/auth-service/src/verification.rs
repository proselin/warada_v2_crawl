use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
};

use crate::{models::Identity, security::bearer_token, state::AppState};

/// Verify an access token and return its subject, principal type, and scopes.
#[utoipa::path(
    get,
    path = "/v1/verify",
    tag = "Verify",
    security(("accessToken" = [])),
    responses(
        (status = 200, description = "Token is valid", body = Identity),
        (status = 401, description = "Missing or invalid token")
    )
)]
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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{models::PrincipalType, test_support::app_state, tokens::issue_token};

    #[tokio::test]
    async fn verification_rejects_missing_or_invalid_tokens_and_caches_valid_claims() {
        let state = app_state().await;
        assert_eq!(
            verify(State(state.clone()), HeaderMap::new())
                .await
                .err()
                .unwrap(),
            StatusCode::UNAUTHORIZED
        );
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, "Basic token".parse().unwrap());
        assert_eq!(
            verify(State(state.clone()), headers.clone())
                .await
                .err()
                .unwrap(),
            StatusCode::UNAUTHORIZED
        );

        headers.insert(AUTHORIZATION, "Bearer invalid.token".parse().unwrap());
        assert_eq!(
            verify(State(state.clone()), headers.clone())
                .await
                .err()
                .unwrap(),
            StatusCode::UNAUTHORIZED
        );

        let token = issue_token(
            &state.jwt_keys,
            &state.issuer,
            &state.audience,
            state.token_ttl,
            "user-1".to_owned(),
            PrincipalType::User,
            vec!["api:read".to_owned(), "api:write".to_owned()],
        )
        .unwrap()
        .access_token;
        headers.insert(AUTHORIZATION, format!("Bearer {token}").parse().unwrap());
        let verified = verify(State(state.clone()), headers.clone())
            .await
            .unwrap()
            .0;
        assert_eq!(verified.subject, "user-1");
        assert_eq!(verified.scopes, vec!["api:read", "api:write"]);
        assert_eq!(
            verify(State(state), headers).await.unwrap().0.subject,
            "user-1"
        );
    }
}
