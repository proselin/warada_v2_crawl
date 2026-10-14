use std::{path::Path, time::Duration};

use tokio_rusqlite::{
    Connection,
    rusqlite::{OptionalExtension, Row, params},
};

use crate::models::{
    AuthorizationCodeRecord, ClientCredential, ClientRecord, UserCredential, UserRecord,
};

pub(crate) struct DatabaseService {
    connection: Connection,
}

impl DatabaseService {
    pub(crate) async fn connect(database_path: &str) -> Result<Self, String> {
        if let Some(parent) = Path::new(database_path).parent()
            && !parent.as_os_str().is_empty()
        {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let connection = Connection::open(database_path)
            .await
            .map_err(|error| error.to_string())?;
        connection
            .call(|connection| {
                connection.busy_timeout(Duration::from_secs(10))?;
                connection.pragma_update(None, "journal_mode", "WAL")?;
                connection.pragma_update(None, "synchronous", "NORMAL")?;
                connection.pragma_update(None, "foreign_keys", "ON")?;
                connection.execute_batch(
                    "CREATE TABLE IF NOT EXISTS users (
                        id TEXT PRIMARY KEY,
                        email TEXT NOT NULL UNIQUE,
                        password_hash TEXT NOT NULL,
                        active INTEGER NOT NULL DEFAULT 1,
                        created_at INTEGER NOT NULL
                    );
                    CREATE TABLE IF NOT EXISTS oauth_clients (
                        client_id TEXT PRIMARY KEY,
                        client_type TEXT NOT NULL,
                        secret_hash TEXT,
                        redirect_uri TEXT,
                        scopes TEXT NOT NULL,
                        active INTEGER NOT NULL DEFAULT 1,
                        created_at INTEGER NOT NULL
                    );
                    CREATE TABLE IF NOT EXISTS authorization_codes (
                        code_hash TEXT PRIMARY KEY,
                        client_id TEXT NOT NULL REFERENCES oauth_clients(client_id),
                        user_id TEXT NOT NULL REFERENCES users(id),
                        redirect_uri TEXT NOT NULL,
                        code_challenge TEXT NOT NULL,
                        scopes TEXT NOT NULL,
                        expires_at INTEGER NOT NULL
                    );",
                )
            })
            .await
            .map_err(|error| error.to_string())?;
        Ok(Self { connection })
    }

    pub(crate) async fn users(
        &self,
    ) -> Result<Vec<UserRecord>, tokio_rusqlite::Error> {
        self.connection
            .call(|connection| {
                let mut statement = connection
                    .prepare("SELECT id, email, active, created_at FROM users ORDER BY created_at DESC")?;
                statement
                    .query_map([], user_record)?
                    .collect::<Result<Vec<_>, _>>()
            })
            .await
    }

    pub(crate) async fn user(
        &self,
        id: &str,
    ) -> Result<Option<UserRecord>, tokio_rusqlite::Error> {
        let id = id.to_owned();
        self.connection
            .call(move |connection| {
                connection
                    .query_row(
                        "SELECT id, email, active, created_at FROM users WHERE id = ?1",
                        [id],
                        user_record,
                    )
                    .optional()
            })
            .await
    }

    pub(crate) async fn user_for_update(
        &self,
        id: &str,
    ) -> Result<Option<UserCredential>, tokio_rusqlite::Error> {
        let id = id.to_owned();
        self.connection
            .call(move |connection| {
                connection
                    .query_row(
                        "SELECT id, email, password_hash, active, created_at
                         FROM users WHERE id = ?1",
                        [id],
                        user_credential,
                    )
                    .optional()
            })
            .await
    }

    pub(crate) async fn user_by_email(
        &self,
        email: &str,
    ) -> Result<Option<UserCredential>, tokio_rusqlite::Error> {
        let email = email.to_owned();
        self.connection
            .call(move |connection| {
                connection
                    .query_row(
                        "SELECT id, email, password_hash, active, created_at
                         FROM users WHERE email = ?1",
                        [email],
                        user_credential,
                    )
                    .optional()
            })
            .await
    }

    pub(crate) async fn create_user(
        &self,
        id: &str,
        email: &str,
        password_hash: &str,
        created_at: i64,
    ) -> Result<(), tokio_rusqlite::Error> {
        let (id, email, password_hash) = (
            id.to_owned(),
            email.to_owned(),
            password_hash.to_owned(),
        );
        self.connection
            .call(move |connection| {
                connection
                    .execute(
                        "INSERT INTO users (id, email, password_hash, active, created_at)
                         VALUES (?1, ?2, ?3, 1, ?4)",
                        params![id, email, password_hash, created_at],
                    )
                    .map(|_| ())
            })
            .await
    }

    pub(crate) async fn update_user(
        &self,
        id: &str,
        email: &str,
        password_hash: &str,
        active: i64,
    ) -> Result<usize, tokio_rusqlite::Error> {
        let (id, email, password_hash) = (
            id.to_owned(),
            email.to_owned(),
            password_hash.to_owned(),
        );
        self.connection
            .call(move |connection| {
                connection.execute(
                    "UPDATE users SET email = ?1, password_hash = ?2, active = ?3
                     WHERE id = ?4",
                    params![email, password_hash, active, id],
                )
            })
            .await
    }

    pub(crate) async fn deactivate_user(
        &self,
        id: &str,
    ) -> Result<usize, tokio_rusqlite::Error> {
        let id = id.to_owned();
        self.connection
            .call(move |connection| {
                connection.execute("UPDATE users SET active = 0 WHERE id = ?1", [id])
            })
            .await
    }

    pub(crate) async fn service_clients(
        &self,
    ) -> Result<Vec<ClientRecord>, tokio_rusqlite::Error> {
        self.connection
            .call(|connection| {
                let mut statement = connection.prepare(
                    "SELECT client_id, scopes, active, created_at FROM oauth_clients
                     WHERE client_type = 'service' ORDER BY created_at DESC",
                )?;
                statement
                    .query_map([], client_record)?
                    .collect::<Result<Vec<_>, _>>()
            })
            .await
    }

    pub(crate) async fn service_client(
        &self,
        client_id: &str,
    ) -> Result<Option<ClientRecord>, tokio_rusqlite::Error> {
        let client_id = client_id.to_owned();
        self.connection
            .call(move |connection| {
                connection
                    .query_row(
                        "SELECT client_id, scopes, active, created_at FROM oauth_clients
                         WHERE client_id = ?1 AND client_type = 'service'",
                        [client_id],
                        client_record,
                    )
                    .optional()
            })
            .await
    }

    pub(crate) async fn create_service_client(
        &self,
        client_id: &str,
        secret_hash: &str,
        scopes: &str,
        created_at: i64,
    ) -> Result<(), tokio_rusqlite::Error> {
        let (client_id, secret_hash, scopes) = (
            client_id.to_owned(),
            secret_hash.to_owned(),
            scopes.to_owned(),
        );
        self.connection
            .call(move |connection| {
                connection
                    .execute(
                        "INSERT INTO oauth_clients
                         (client_id, client_type, secret_hash, redirect_uri, scopes, active, created_at)
                         VALUES (?1, 'service', ?2, NULL, ?3, 1, ?4)",
                        params![client_id, secret_hash, scopes, created_at],
                    )
                    .map(|_| ())
            })
            .await
    }

    pub(crate) async fn create_public_client(
        &self,
        client_id: &str,
        redirect_uri: &str,
        scopes: &str,
        created_at: i64,
    ) -> Result<(), tokio_rusqlite::Error> {
        let (client_id, redirect_uri, scopes) = (
            client_id.to_owned(),
            redirect_uri.to_owned(),
            scopes.to_owned(),
        );
        self.connection
            .call(move |connection| {
                connection
                    .execute(
                        "INSERT INTO oauth_clients
                         (client_id, client_type, secret_hash, redirect_uri, scopes, active, created_at)
                         VALUES (?1, 'public', NULL, ?2, ?3, 1, ?4)",
                        params![client_id, redirect_uri, scopes, created_at],
                    )
                    .map(|_| ())
            })
            .await
    }

    pub(crate) async fn client(
        &self,
        client_id: &str,
    ) -> Result<Option<ClientCredential>, tokio_rusqlite::Error> {
        let client_id = client_id.to_owned();
        self.connection
            .call(move |connection| {
                connection
                    .query_row(
                        "SELECT client_id, client_type, secret_hash, redirect_uri, scopes, active
                         FROM oauth_clients WHERE client_id = ?1",
                        [client_id],
                        client_credential,
                    )
                    .optional()
            })
            .await
    }

    pub(crate) async fn update_service_client(
        &self,
        client_id: &str,
        scopes: &str,
        active: i64,
    ) -> Result<usize, tokio_rusqlite::Error> {
        let (client_id, scopes) = (client_id.to_owned(), scopes.to_owned());
        self.connection
            .call(move |connection| {
                connection.execute(
                    "UPDATE oauth_clients SET scopes = ?1, active = ?2
                     WHERE client_id = ?3 AND client_type = 'service'",
                    params![scopes, active, client_id],
                )
            })
            .await
    }

    pub(crate) async fn deactivate_service_client(
        &self,
        client_id: &str,
    ) -> Result<usize, tokio_rusqlite::Error> {
        let client_id = client_id.to_owned();
        self.connection
            .call(move |connection| {
                connection.execute(
                    "UPDATE oauth_clients SET active = 0
                     WHERE client_id = ?1 AND client_type = 'service'",
                    [client_id],
                )
            })
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
    ) -> Result<(), tokio_rusqlite::Error> {
        let (code_hash, client_id, user_id, redirect_uri, code_challenge, scopes) = (
            code_hash.to_owned(),
            client_id.to_owned(),
            user_id.to_owned(),
            redirect_uri.to_owned(),
            code_challenge.to_owned(),
            scopes.to_owned(),
        );
        self.connection
            .call(move |connection| {
                connection
                    .execute(
                        "INSERT INTO authorization_codes
                         (code_hash, client_id, user_id, redirect_uri, code_challenge, scopes, expires_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                        params![
                            code_hash,
                            client_id,
                            user_id,
                            redirect_uri,
                            code_challenge,
                            scopes,
                            expires_at,
                        ],
                    )
                    .map(|_| ())
            })
            .await
    }

    pub(crate) async fn redeem_authorization_code(
        &self,
        code_hash: &str,
        client_id: &str,
        redirect_uri: &str,
        now: i64,
    ) -> Result<Option<AuthorizationCodeRecord>, tokio_rusqlite::Error> {
        let (code_hash, client_id, redirect_uri) = (
            code_hash.to_owned(),
            client_id.to_owned(),
            redirect_uri.to_owned(),
        );
        self.connection
            .call(move |connection| {
                connection
                    .query_row(
                        "DELETE FROM authorization_codes
                         WHERE code_hash = ?1 AND client_id = ?2 AND redirect_uri = ?3
                           AND expires_at > ?4
                         RETURNING user_id, code_challenge, scopes",
                        params![code_hash, client_id, redirect_uri, now],
                        authorization_code,
                    )
                    .optional()
            })
            .await
    }

    pub(crate) async fn user_is_active(
        &self,
        user_id: &str,
    ) -> Result<bool, tokio_rusqlite::Error> {
        let user_id = user_id.to_owned();
        self.connection
            .call(move |connection| {
                connection
                    .query_row("SELECT active FROM users WHERE id = ?1", [user_id], |row| {
                        row.get::<_, i64>(0)
                    })
                    .optional()
                    .map(|active| active == Some(1))
            })
            .await
    }

    pub(crate) async fn update_password_hash(
        &self,
        user_id: &str,
        password_hash: &str,
    ) -> Result<usize, tokio_rusqlite::Error> {
        let (user_id, password_hash) = (user_id.to_owned(), password_hash.to_owned());
        self.connection
            .call(move |connection| {
                connection.execute(
                    "UPDATE users SET password_hash = ?1 WHERE id = ?2",
                    params![password_hash, user_id],
                )
            })
            .await
    }
}

fn user_record(row: &Row<'_>) -> tokio_rusqlite::rusqlite::Result<UserRecord> {
    Ok(UserRecord {
        id: row.get("id")?,
        email: row.get("email")?,
        active: row.get("active")?,
        created_at: row.get("created_at")?,
    })
}

fn user_credential(row: &Row<'_>) -> tokio_rusqlite::rusqlite::Result<UserCredential> {
    Ok(UserCredential {
        id: row.get("id")?,
        email: row.get("email")?,
        password_hash: row.get("password_hash")?,
        active: row.get("active")?,
        created_at: row.get("created_at")?,
    })
}

fn client_record(row: &Row<'_>) -> tokio_rusqlite::rusqlite::Result<ClientRecord> {
    Ok(ClientRecord {
        client_id: row.get("client_id")?,
        scopes: row.get("scopes")?,
        active: row.get("active")?,
        created_at: row.get("created_at")?,
    })
}

fn client_credential(row: &Row<'_>) -> tokio_rusqlite::rusqlite::Result<ClientCredential> {
    Ok(ClientCredential {
        client_id: row.get("client_id")?,
        client_type: row.get("client_type")?,
        secret_hash: row.get("secret_hash")?,
        redirect_uri: row.get("redirect_uri")?,
        scopes: row.get("scopes")?,
        active: row.get("active")?,
    })
}

fn authorization_code(
    row: &Row<'_>,
) -> tokio_rusqlite::rusqlite::Result<AuthorizationCodeRecord> {
    Ok(AuthorizationCodeRecord {
        user_id: row.get("user_id")?,
        code_challenge: row.get("code_challenge")?,
        scopes: row.get("scopes")?,
    })
}
