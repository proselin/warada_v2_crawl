use tokio_postgres::{Client, NoTls, Row};

use crate::models::{
    AuthorizationCodeRecord, ClientCredential, ClientRecord, UserCredential, UserRecord,
};

pub(crate) struct DatabaseService {
    client: Client,
}

impl DatabaseService {
    pub(crate) async fn connect(database_url: &str) -> Result<Self, tokio_postgres::Error> {
        let (client, connection) = tokio_postgres::connect(database_url, NoTls).await?;
        tokio::spawn(async move {
            if let Err(error) = connection.await {
                tracing::error!(%error, "postgres connection failed");
            }
        });
        client
            .batch_execute(
                "CREATE TABLE IF NOT EXISTS users (
                    id TEXT PRIMARY KEY,
                    email TEXT NOT NULL UNIQUE,
                    password_hash TEXT NOT NULL,
                    active BIGINT NOT NULL DEFAULT 1,
                    created_at BIGINT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS oauth_clients (
                    client_id TEXT PRIMARY KEY,
                    client_type TEXT NOT NULL,
                    secret_hash TEXT,
                    redirect_uri TEXT,
                    scopes TEXT NOT NULL,
                    active BIGINT NOT NULL DEFAULT 1,
                    created_at BIGINT NOT NULL
                );
                CREATE TABLE IF NOT EXISTS authorization_codes (
                    code_hash TEXT PRIMARY KEY,
                    client_id TEXT NOT NULL REFERENCES oauth_clients(client_id),
                    user_id TEXT NOT NULL REFERENCES users(id),
                    redirect_uri TEXT NOT NULL,
                    code_challenge TEXT NOT NULL,
                    scopes TEXT NOT NULL,
                    expires_at BIGINT NOT NULL
                );",
            )
            .await?;
        Ok(Self { client })
    }

    pub(crate) async fn users(&self) -> Result<Vec<UserRecord>, tokio_postgres::Error> {
        self.client
            .query(
                "SELECT id, email, active, created_at FROM users ORDER BY created_at DESC",
                &[],
            )
            .await
            .map(|rows| rows.into_iter().map(user_record).collect())
    }

    pub(crate) async fn user(&self, id: &str) -> Result<Option<UserRecord>, tokio_postgres::Error> {
        self.client
            .query_opt(
                "SELECT id, email, active, created_at FROM users WHERE id = $1",
                &[&id],
            )
            .await
            .map(|row| row.map(user_record))
    }

    pub(crate) async fn user_for_update(
        &self,
        id: &str,
    ) -> Result<Option<UserCredential>, tokio_postgres::Error> {
        self.client
            .query_opt(
                "SELECT id, email, password_hash, active, created_at FROM users WHERE id = $1",
                &[&id],
            )
            .await
            .map(|row| row.map(user_credential))
    }

    pub(crate) async fn user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<UserCredential>, tokio_postgres::Error> {
        self.client
            .query_opt(
                "SELECT id, email, password_hash, active, created_at FROM users WHERE email = $1",
                &[&email],
            )
            .await
            .map(|row| row.map(user_credential))
    }

    pub(crate) async fn create_user(
        &self,
        id: &str,
        email: &str,
        password_hash: &str,
        created_at: i64,
    ) -> Result<(), tokio_postgres::Error> {
        self.client
            .execute(
                "INSERT INTO users (id, email, password_hash, active, created_at) VALUES ($1, $2, $3, 1, $4)",
                &[&id, &email, &password_hash, &created_at],
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn update_user(
        &self,
        id: &str,
        email: &str,
        password_hash: &str,
        active: i64,
    ) -> Result<u64, tokio_postgres::Error> {
        self.client
            .execute(
                "UPDATE users SET email = $1, password_hash = $2, active = $3 WHERE id = $4",
                &[&email, &password_hash, &active, &id],
            )
            .await
    }

    pub(crate) async fn deactivate_user(&self, id: &str) -> Result<u64, tokio_postgres::Error> {
        self.client
            .execute("UPDATE users SET active = 0 WHERE id = $1", &[&id])
            .await
    }

    pub(crate) async fn service_clients(&self) -> Result<Vec<ClientRecord>, tokio_postgres::Error> {
        self.client
            .query(
                "SELECT client_id, scopes, active, created_at FROM oauth_clients WHERE client_type = 'service' ORDER BY created_at DESC",
                &[],
            )
            .await
            .map(|rows| rows.into_iter().map(client_record).collect())
    }

    pub(crate) async fn service_client(
        &self,
        client_id: &str,
    ) -> Result<Option<ClientRecord>, tokio_postgres::Error> {
        self.client
            .query_opt(
                "SELECT client_id, scopes, active, created_at FROM oauth_clients WHERE client_id = $1 AND client_type = 'service'",
                &[&client_id],
            )
            .await
            .map(|row| row.map(client_record))
    }

    pub(crate) async fn create_service_client(
        &self,
        client_id: &str,
        secret_hash: &str,
        scopes: &str,
        created_at: i64,
    ) -> Result<(), tokio_postgres::Error> {
        self.client
            .execute(
                "INSERT INTO oauth_clients (client_id, client_type, secret_hash, redirect_uri, scopes, active, created_at) VALUES ($1, 'service', $2, NULL, $3, 1, $4)",
                &[&client_id, &secret_hash, &scopes, &created_at],
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn create_public_client(
        &self,
        client_id: &str,
        redirect_uri: &str,
        scopes: &str,
        created_at: i64,
    ) -> Result<(), tokio_postgres::Error> {
        self.client
            .execute(
                "INSERT INTO oauth_clients (client_id, client_type, secret_hash, redirect_uri, scopes, active, created_at) VALUES ($1, 'public', NULL, $2, $3, 1, $4)",
                &[&client_id, &redirect_uri, &scopes, &created_at],
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn client(
        &self,
        client_id: &str,
    ) -> Result<Option<ClientCredential>, tokio_postgres::Error> {
        self.client
            .query_opt(
                "SELECT client_id, client_type, secret_hash, redirect_uri, scopes, active FROM oauth_clients WHERE client_id = $1",
                &[&client_id],
            )
            .await
            .map(|row| row.map(client_credential))
    }

    pub(crate) async fn update_service_client(
        &self,
        client_id: &str,
        scopes: &str,
        active: i64,
    ) -> Result<u64, tokio_postgres::Error> {
        self.client
            .execute(
                "UPDATE oauth_clients SET scopes = $1, active = $2 WHERE client_id = $3 AND client_type = 'service'",
                &[&scopes, &active, &client_id],
            )
            .await
    }

    pub(crate) async fn deactivate_service_client(
        &self,
        client_id: &str,
    ) -> Result<u64, tokio_postgres::Error> {
        self.client
            .execute(
                "UPDATE oauth_clients SET active = 0 WHERE client_id = $1 AND client_type = 'service'",
                &[&client_id],
            )
            .await
    }

    pub(crate) async fn insert_authorization_code(
        &self,
        code_hash: &str,
        client_id: &str,
        user_id: &str,
        redirect_uri: &str,
        code_challenge: &str,
        scopes: &str,
        expires_at: i64,
    ) -> Result<(), tokio_postgres::Error> {
        self.client
            .execute(
                "INSERT INTO authorization_codes (code_hash, client_id, user_id, redirect_uri, code_challenge, scopes, expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                &[&code_hash, &client_id, &user_id, &redirect_uri, &code_challenge, &scopes, &expires_at],
            )
            .await
            .map(|_| ())
    }

    pub(crate) async fn redeem_authorization_code(
        &self,
        code_hash: &str,
        client_id: &str,
        redirect_uri: &str,
        now: i64,
    ) -> Result<Option<AuthorizationCodeRecord>, tokio_postgres::Error> {
        self.client
            .query_opt(
                "DELETE FROM authorization_codes WHERE code_hash = $1 AND client_id = $2 AND redirect_uri = $3 AND expires_at > $4 RETURNING user_id, code_challenge, scopes",
                &[&code_hash, &client_id, &redirect_uri, &now],
            )
            .await
            .map(|row| row.map(authorization_code))
    }

    pub(crate) async fn user_is_active(
        &self,
        user_id: &str,
    ) -> Result<bool, tokio_postgres::Error> {
        self.client
            .query_opt("SELECT active FROM users WHERE id = $1", &[&user_id])
            .await
            .map(|row| row.is_some_and(|row| row.get::<_, i64>("active") == 1))
    }

    pub(crate) async fn update_password_hash(
        &self,
        user_id: &str,
        password_hash: &str,
    ) -> Result<u64, tokio_postgres::Error> {
        self.client
            .execute(
                "UPDATE users SET password_hash = $1 WHERE id = $2",
                &[&password_hash, &user_id],
            )
            .await
    }
}

fn user_record(row: Row) -> UserRecord {
    UserRecord {
        id: row.get("id"),
        email: row.get("email"),
        active: row.get("active"),
        created_at: row.get("created_at"),
    }
}

fn user_credential(row: Row) -> UserCredential {
    UserCredential {
        id: row.get("id"),
        email: row.get("email"),
        password_hash: row.get("password_hash"),
        active: row.get("active"),
        created_at: row.get("created_at"),
    }
}

fn client_record(row: Row) -> ClientRecord {
    ClientRecord {
        client_id: row.get("client_id"),
        scopes: row.get("scopes"),
        active: row.get("active"),
        created_at: row.get("created_at"),
    }
}

fn client_credential(row: Row) -> ClientCredential {
    ClientCredential {
        client_id: row.get("client_id"),
        client_type: row.get("client_type"),
        secret_hash: row.get("secret_hash"),
        redirect_uri: row.get("redirect_uri"),
        scopes: row.get("scopes"),
        active: row.get("active"),
    }
}

fn authorization_code(row: Row) -> AuthorizationCodeRecord {
    AuthorizationCodeRecord {
        user_id: row.get("user_id"),
        code_challenge: row.get("code_challenge"),
        scopes: row.get("scopes"),
    }
}
