use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PrincipalType {
    User,
    Service,
}

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Claims {
    pub(crate) sub: String,
    pub(crate) iss: String,
    pub(crate) aud: String,
    pub(crate) exp: u64,
    pub(crate) iat: u64,
    pub(crate) principal_type: PrincipalType,
    pub(crate) scope: Option<String>,
}

#[derive(Serialize)]
pub(crate) struct Identity {
    pub(crate) subject: String,
    pub(crate) principal_type: PrincipalType,
    pub(crate) scopes: Vec<String>,
}

pub(crate) struct UserRecord {
    pub(crate) id: String,
    pub(crate) email: String,
    pub(crate) active: i64,
    pub(crate) created_at: i64,
}

#[derive(Serialize)]
pub(crate) struct UserView {
    pub(crate) id: String,
    pub(crate) email: String,
    pub(crate) active: bool,
    pub(crate) created_at: i64,
}

impl From<UserRecord> for UserView {
    fn from(user: UserRecord) -> Self {
        Self {
            id: user.id,
            email: user.email,
            active: user.active != 0,
            created_at: user.created_at,
        }
    }
}

pub(crate) struct UserCredential {
    pub(crate) id: String,
    pub(crate) email: String,
    pub(crate) password_hash: String,
    pub(crate) active: i64,
    pub(crate) created_at: i64,
}

#[derive(Deserialize)]
pub(crate) struct CreateUser {
    pub(crate) email: String,
    pub(crate) password: String,
}

#[derive(Deserialize)]
pub(crate) struct UpdateUser {
    pub(crate) email: Option<String>,
    pub(crate) password: Option<String>,
    pub(crate) active: Option<bool>,
}

pub(crate) struct ClientRecord {
    pub(crate) client_id: String,
    pub(crate) scopes: String,
    pub(crate) active: i64,
    pub(crate) created_at: i64,
}

#[derive(Serialize)]
pub(crate) struct ClientView {
    pub(crate) client_id: String,
    pub(crate) scopes: String,
    pub(crate) active: bool,
    pub(crate) created_at: i64,
}

impl From<ClientRecord> for ClientView {
    fn from(client: ClientRecord) -> Self {
        Self {
            client_id: client.client_id,
            scopes: client.scopes,
            active: client.active != 0,
            created_at: client.created_at,
        }
    }
}

pub(crate) struct ClientCredential {
    pub(crate) client_id: String,
    pub(crate) client_type: String,
    pub(crate) secret_hash: Option<String>,
    pub(crate) redirect_uri: Option<String>,
    pub(crate) scopes: String,
    pub(crate) active: i64,
}

pub(crate) struct AuthorizationCodeRecord {
    pub(crate) user_id: String,
    pub(crate) code_challenge: String,
    pub(crate) scopes: String,
}

#[derive(Deserialize)]
pub(crate) struct CreateClient {
    pub(crate) client_id: String,
    pub(crate) scopes: Option<Vec<String>>,
}

#[derive(Serialize)]
pub(crate) struct NewClientResponse {
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
    pub(crate) scopes: Vec<String>,
}

#[derive(Deserialize)]
pub(crate) struct UpdateClient {
    pub(crate) scopes: Option<Vec<String>>,
    pub(crate) active: Option<bool>,
}

#[derive(Serialize)]
pub(crate) struct TokenResponse {
    pub(crate) access_token: String,
    pub(crate) token_type: &'static str,
    pub(crate) expires_in: u64,
}
