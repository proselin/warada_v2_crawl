use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
};

use crate::{
    models::{
        ClientView, CreateClient, CreateUser, NewClientResponse, UpdateClient, UpdateUser, UserView,
    },
    security::{
        hash_password, hash_secret, is_unique_violation, normalize_email, normalize_scopes,
        random_token, require_admin, unix_now, validate_password,
    },
    state::AppState,
};

pub(crate) async fn list_users(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserView>>, StatusCode> {
    require_admin(&headers, &state.admin_key)?;
    let users = state
        .db
        .users()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(users.into_iter().map(UserView::from).collect()))
}

pub(crate) async fn create_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateUser>,
) -> Result<(StatusCode, Json<UserView>), StatusCode> {
    require_admin(&headers, &state.admin_key)?;
    let email = normalize_email(input.email)?;
    validate_password(&input.password)?;
    let id = random_token(16)?;
    let password_hash = hash_password(input.password, state.password_pepper.clone()).await?;
    let created_at = unix_now()?;
    if let Err(error) = state
        .db
        .create_user(&id, &email, &password_hash, created_at)
        .await
    {
        return Err(if is_unique_violation(&error) {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        });
    }
    Ok((
        StatusCode::CREATED,
        Json(UserView {
            id,
            email,
            active: true,
            created_at,
        }),
    ))
}

pub(crate) async fn get_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<UserView>, StatusCode> {
    require_admin(&headers, &state.admin_key)?;
    let user = state
        .db
        .user(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(UserView::from(user)))
}

pub(crate) async fn update_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(input): Json<UpdateUser>,
) -> Result<Json<UserView>, StatusCode> {
    require_admin(&headers, &state.admin_key)?;
    if input.email.is_none() && input.password.is_none() && input.active.is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let current = state
        .db
        .user_for_update(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let email = input
        .email
        .map(normalize_email)
        .transpose()?
        .unwrap_or(current.email);
    let password_hash = match input.password {
        Some(password) => {
            validate_password(&password)?;
            hash_password(password, state.password_pepper.clone()).await?
        }
        None => current.password_hash,
    };
    let active = input
        .active
        .map(|active| active as i64)
        .unwrap_or(current.active);
    if let Err(error) = state
        .db
        .update_user(&id, &email, &password_hash, active)
        .await
    {
        return Err(if is_unique_violation(&error) {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        });
    }
    Ok(Json(UserView {
        id,
        email,
        active: active != 0,
        created_at: current.created_at,
    }))
}

pub(crate) async fn deactivate_user(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    require_admin(&headers, &state.admin_key)?;
    let updated = state
        .db
        .deactivate_user(&id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if updated == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn list_clients(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ClientView>>, StatusCode> {
    require_admin(&headers, &state.admin_key)?;
    let clients = state
        .db
        .service_clients()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(clients.into_iter().map(ClientView::from).collect()))
}

pub(crate) async fn create_client(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreateClient>,
) -> Result<(StatusCode, Json<NewClientResponse>), StatusCode> {
    require_admin(&headers, &state.admin_key)?;
    if !(3..=100).contains(&input.client_id.len())
        || !input
            .client_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    let scopes = normalize_scopes(input.scopes.unwrap_or_else(|| vec!["api:read".to_owned()]))?;
    let secret = random_token(32)?;
    let secret_hash = hash_secret(secret.clone()).await?;
    let created_at = unix_now()?;
    if let Err(error) = state
        .db
        .create_service_client(
            &input.client_id,
            &secret_hash,
            &scopes.join(" "),
            created_at,
        )
        .await
    {
        return Err(if is_unique_violation(&error) {
            StatusCode::CONFLICT
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        });
    }
    Ok((
        StatusCode::CREATED,
        Json(NewClientResponse {
            client_id: input.client_id,
            client_secret: secret,
            scopes,
        }),
    ))
}

pub(crate) async fn update_client(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(client_id): Path<String>,
    Json(input): Json<UpdateClient>,
) -> Result<Json<ClientView>, StatusCode> {
    require_admin(&headers, &state.admin_key)?;
    if input.active.is_none() && input.scopes.is_none() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let current = state
        .db
        .service_client(&client_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    let scopes = match input.scopes {
        Some(scopes) => normalize_scopes(scopes)?.join(" "),
        None => current.scopes,
    };
    let active = input
        .active
        .map(|active| active as i64)
        .unwrap_or(current.active);
    let updated = state
        .db
        .update_service_client(&client_id, &scopes, active)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if updated == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(Json(ClientView {
        client_id,
        scopes,
        active: active != 0,
        created_at: current.created_at,
    }))
}

pub(crate) async fn deactivate_client(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(client_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    require_admin(&headers, &state.admin_key)?;
    let updated = state
        .db
        .deactivate_service_client(&client_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if updated == 0 {
        return Err(StatusCode::NOT_FOUND);
    }
    Ok(StatusCode::NO_CONTENT)
}
