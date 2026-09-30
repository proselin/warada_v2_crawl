use std::sync::Arc;

use axum::{
    Form, Json,
    extract::{Query, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    response::{Html, IntoResponse, Redirect},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use url::Url;

use crate::{
    models::{ClientCredential, PrincipalType, TokenResponse},
    security::{
        PasswordCheck, hash_password, is_unique_violation, normalize_email, normalize_scopes,
        random_token, require_admin, split_scopes, unix_now, verify_password, verify_secret,
    },
    state::AppState,
    tokens::issue_token,
};

#[derive(Deserialize)]
pub(super) struct CreatePublicClient {
    client_id: String,
    redirect_uri: String,
    scopes: Option<Vec<String>>,
}

#[derive(Deserialize, Clone)]
pub(super) struct AuthorizeRequest {
    response_type: String,
    client_id: String,
    redirect_uri: String,
    state: String,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    scope: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct AuthorizeForm {
    csrf: String,
    response_type: String,
    client_id: String,
    redirect_uri: String,
    state: String,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    scope: String,
    email: String,
    password: String,
    consent: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct TokenRequest {
    grant_type: String,
    client_id: Option<String>,
    client_secret: Option<String>,
    scope: Option<String>,
    code: Option<String>,
    redirect_uri: Option<String>,
    code_verifier: Option<String>,
}

#[derive(Serialize)]
pub(super) struct PublicClientResponse {
    client_id: String,
    redirect_uri: String,
    scopes: Vec<String>,
    active: bool,
    created_at: i64,
}

pub(super) async fn create_public_client(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(input): Json<CreatePublicClient>,
) -> Result<(StatusCode, Json<PublicClientResponse>), StatusCode> {
    require_admin(&headers, &state.admin_key)?;
    validate_client_id(&input.client_id)?;
    if !valid_redirect_uri(&input.redirect_uri) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let scopes = normalize_scopes(input.scopes.unwrap_or_else(|| vec!["api:read".to_owned()]))?;
    let created_at = unix_now()?;
    let scope_text = scopes.join(" ");
    if let Err(error) = state
        .db
        .create_public_client(
            &input.client_id,
            &input.redirect_uri,
            &scope_text,
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
        Json(PublicClientResponse {
            client_id: input.client_id,
            redirect_uri: input.redirect_uri,
            scopes,
            active: true,
            created_at,
        }),
    ))
}

pub(super) async fn authorize_page(
    State(state): State<Arc<AppState>>,
    Query(request): Query<AuthorizeRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let (client, scopes) = validate_authorize_request(&state, &request).await?;
    let csrf = random_token(32)?;
    let mut headers = HeaderMap::new();
    let secure = if state.cookie_secure { "; Secure" } else { "" };
    let cookie = format!(
        "auth_csrf={csrf}; Path=/oauth/authorize; HttpOnly; SameSite=Lax; Max-Age=300{secure}"
    );
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );

    let html = render_template(
        include_str!("../assets/authorize.html"),
        &[
            ("CLIENT_NAME", escape_html(&client.client_id)),
            ("SCOPES", escape_html(&scopes.join(" "))),
            ("CSRF", escape_html(&csrf)),
            ("RESPONSE_TYPE", escape_html(&request.response_type)),
            ("CLIENT_ID", escape_html(&request.client_id)),
            ("REDIRECT_URI", escape_html(&request.redirect_uri)),
            ("STATE", escape_html(&request.state)),
            (
                "CODE_CHALLENGE",
                escape_html(request.code_challenge.as_deref().unwrap_or_default()),
            ),
            (
                "CODE_CHALLENGE_METHOD",
                escape_html(request.code_challenge_method.as_deref().unwrap_or_default()),
            ),
        ],
    );
    Ok((headers, Html(html)))
}

pub(super) async fn authorize(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Form(form): Form<AuthorizeForm>,
) -> Result<impl IntoResponse, StatusCode> {
    let cookie_csrf = cookie_value(&headers, "auth_csrf").ok_or(StatusCode::BAD_REQUEST)?;
    if !bool::from(cookie_csrf.as_bytes().ct_eq(form.csrf.as_bytes()))
        || form.consent.as_deref() != Some("approve")
    {
        return Err(StatusCode::FORBIDDEN);
    }

    let request: AuthorizeRequest = AuthorizeRequest {
        response_type: form.response_type,
        client_id: form.client_id,
        redirect_uri: form.redirect_uri,
        state: form.state,
        code_challenge: form.code_challenge,
        code_challenge_method: form.code_challenge_method,
        scope: Some(form.scope),
    };
    let (client, scopes) = validate_authorize_request(&state, &request).await?;
    let email = normalize_email(form.email)?;
    let user = state
        .db
        .user_by_email(&email)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if user.active == 0 {
        return Err(StatusCode::UNAUTHORIZED);
    }
    match verify_password(
        form.password.clone(),
        user.password_hash.clone(),
        state.password_pepper.clone(),
    )
    .await?
    {
        PasswordCheck::Invalid => return Err(StatusCode::UNAUTHORIZED),
        PasswordCheck::Valid => {}
        PasswordCheck::ValidNeedsRehash => {
            let password_hash =
                hash_password(form.password.clone(), state.password_pepper.clone()).await?;
            state
                .db
                .update_password_hash(&user.id, &password_hash)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
    }

    let mut redirect = Url::parse(&request.redirect_uri).map_err(|_| StatusCode::BAD_REQUEST)?;
    if request.response_type == "code" {
        let code = random_token(32)?;
        let expires_at = unix_now()?.saturating_add(300);
        let code_hash = hash_code(&code);
        let code_challenge = request
            .code_challenge
            .as_deref()
            .ok_or(StatusCode::BAD_REQUEST)?;
        let scope_text = scopes.join(" ");
        state
            .db
            .insert_authorization_code(
                &code_hash,
                &client.client_id,
                &user.id,
                &request.redirect_uri,
                code_challenge,
                &scope_text,
                expires_at,
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        redirect
            .query_pairs_mut()
            .append_pair("code", &code)
            .append_pair("state", &request.state);
    } else {
        let token = issue_token(
            &state.jwt_keys,
            &state.issuer,
            &state.audience,
            state.token_ttl,
            user.id,
            PrincipalType::User,
            scopes,
        )?;
        let fragment = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("access_token", &token.access_token)
            .append_pair("token_type", token.token_type)
            .append_pair("expires_in", &token.expires_in.to_string())
            .append_pair("state", &request.state)
            .finish();
        redirect.set_fragment(Some(&fragment));
    }

    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    let secure = if state.cookie_secure { "; Secure" } else { "" };
    let clear_cookie =
        format!("auth_csrf=; Path=/oauth/authorize; HttpOnly; SameSite=Lax; Max-Age=0{secure}");
    headers.insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&clear_cookie).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
    );
    Ok((headers, Redirect::to(redirect.as_str())))
}

pub(super) async fn token(
    State(state): State<Arc<AppState>>,
    Form(request): Form<TokenRequest>,
) -> Result<Json<TokenResponse>, StatusCode> {
    match request.grant_type.as_str() {
        "client_credentials" => issue_service_token(&state, request).await.map(Json),
        "authorization_code" => exchange_code(&state, request).await.map(Json),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}

async fn issue_service_token(
    state: &AppState,
    request: TokenRequest,
) -> Result<TokenResponse, StatusCode> {
    let client_id = request.client_id.ok_or(StatusCode::BAD_REQUEST)?;
    let secret = request.client_secret.ok_or(StatusCode::UNAUTHORIZED)?;
    let client = state
        .db
        .client(&client_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let secret_hash = client.secret_hash.ok_or(StatusCode::UNAUTHORIZED)?;
    if client.active == 0
        || client.client_type != "service"
        || !verify_secret(secret, secret_hash).await?
    {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let allowed = split_scopes(&client.scopes);
    let scopes = requested_scopes(request.scope, &allowed)?;
    issue_token(
        &state.jwt_keys,
        &state.issuer,
        &state.audience,
        state.token_ttl,
        client.client_id,
        PrincipalType::Service,
        scopes,
    )
}

async fn exchange_code(
    state: &AppState,
    request: TokenRequest,
) -> Result<TokenResponse, StatusCode> {
    let client_id = request.client_id.ok_or(StatusCode::BAD_REQUEST)?;
    let code = request.code.ok_or(StatusCode::BAD_REQUEST)?;
    let verifier = request.code_verifier.ok_or(StatusCode::BAD_REQUEST)?;
    let redirect_uri = request.redirect_uri.ok_or(StatusCode::BAD_REQUEST)?;
    if !valid_code_verifier(&verifier) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let client = load_public_client(state, &client_id, &redirect_uri).await?;
    if client.secret_hash.is_some() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let code_hash = hash_code(&code);
    let now = unix_now()?;
    let code_record = state
        .db
        .redeem_authorization_code(&code_hash, &client_id, &redirect_uri, now)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::BAD_REQUEST)?;
    if pkce_challenge(&verifier) != code_record.code_challenge {
        return Err(StatusCode::BAD_REQUEST);
    }
    let active = state
        .db
        .user_is_active(&code_record.user_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if !active {
        return Err(StatusCode::UNAUTHORIZED);
    }
    issue_token(
        &state.jwt_keys,
        &state.issuer,
        &state.audience,
        state.token_ttl,
        code_record.user_id,
        PrincipalType::User,
        split_scopes(&code_record.scopes),
    )
}

async fn validate_authorize_request(
    state: &AppState,
    request: &AuthorizeRequest,
) -> Result<(ClientCredential, Vec<String>), StatusCode> {
    validate_flow_policy(
        &request.response_type,
        &request.state,
        request.code_challenge.as_deref(),
        request.code_challenge_method.as_deref(),
        state.allow_implicit,
    )?;
    let client = load_public_client(state, &request.client_id, &request.redirect_uri).await?;
    let allowed = split_scopes(&client.scopes);
    let scopes = requested_scopes(request.scope.clone(), &allowed)?;
    Ok((client, scopes))
}

pub(crate) fn validate_flow_policy(
    response_type: &str,
    state: &str,
    code_challenge: Option<&str>,
    code_challenge_method: Option<&str>,
    allow_implicit: bool,
) -> Result<(), StatusCode> {
    if state.is_empty() || state.len() > 512 {
        return Err(StatusCode::BAD_REQUEST);
    }
    match response_type {
        "code" => {
            if code_challenge_method != Some("S256")
                || !code_challenge.is_some_and(valid_code_challenge)
            {
                return Err(StatusCode::BAD_REQUEST);
            }
        }
        "token" if allow_implicit => {}
        "token" => return Err(StatusCode::BAD_REQUEST),
        _ => return Err(StatusCode::BAD_REQUEST),
    }
    Ok(())
}

async fn load_public_client(
    state: &AppState,
    client_id: &str,
    redirect_uri: &str,
) -> Result<ClientCredential, StatusCode> {
    let client = state
        .db
        .client(client_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::BAD_REQUEST)?;
    if client.active == 0
        || client.client_type != "public"
        || client.redirect_uri.as_deref() != Some(redirect_uri)
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(client)
}

fn requested_scopes(
    requested: Option<String>,
    allowed: &[String],
) -> Result<Vec<String>, StatusCode> {
    let scopes = match requested {
        Some(scopes) => {
            normalize_scopes(scopes.split_ascii_whitespace().map(str::to_owned).collect())?
        }
        None => allowed.to_vec(),
    };
    if scopes.is_empty() || scopes.iter().any(|scope| !allowed.contains(scope)) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(scopes)
}

pub(crate) fn valid_redirect_uri(value: &str) -> bool {
    let Ok(url) = Url::parse(value) else {
        return false;
    };
    let local_http = url.scheme() == "http"
        && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    (url.scheme() == "https" || local_http)
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none()
}

fn validate_client_id(client_id: &str) -> Result<(), StatusCode> {
    if !(3..=100).contains(&client_id.len())
        || !client_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(StatusCode::BAD_REQUEST);
    }
    Ok(())
}

pub(crate) fn valid_code_challenge(challenge: &str) -> bool {
    challenge.len() == 43
        && URL_SAFE_NO_PAD
            .decode(challenge)
            .is_ok_and(|decoded| decoded.len() == 32)
}

pub(crate) fn valid_code_verifier(verifier: &str) -> bool {
    (43..=128).contains(&verifier.len())
        && verifier
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~'))
}

pub(crate) fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub(crate) fn hash_code(code: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(code.as_bytes()))
}

fn cookie_value<'a>(headers: &'a HeaderMap, key: &str) -> Option<&'a str> {
    headers
        .get(header::COOKIE)?
        .to_str()
        .ok()?
        .split(';')
        .filter_map(|cookie| cookie.trim().split_once('='))
        .find_map(|(name, value)| (name == key).then_some(value))
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

pub(crate) fn render_template(template: &str, values: &[(&str, String)]) -> String {
    let mut rendered = String::with_capacity(template.len());
    let mut remaining = template;

    while let Some(start) = remaining.find("{{") {
        rendered.push_str(&remaining[..start]);
        remaining = &remaining[start..];
        let Some(end) = remaining.find("}}") else {
            rendered.push_str(remaining);
            return rendered;
        };
        let name = &remaining[2..end];
        if let Some((_, value)) = values.iter().find(|(key, _)| *key == name) {
            rendered.push_str(value);
        } else {
            rendered.push_str(&remaining[..end + 2]);
        }
        remaining = &remaining[end + 2..];
    }
    rendered.push_str(remaining);
    rendered
}
