# Auth Service

Axum/Tokio authentication service with PostgreSQL storage through
`tokio-postgres`. It manages users and OAuth clients, issues RS256 access
tokens, and verifies tokens for downstream services.

## Run

Provide an RS256 signing key, a directory of public keys named by key ID, plus
an admin API key of at least 32 bytes:

```sh
DATABASE_URL='postgres://auth:password@localhost/auth' \
JWT_PRIVATE_KEY_PATH=/path/to/private.pem \
JWT_PUBLIC_KEYS_DIR=/path/to/public-keys \
JWT_SIGNING_KEY_ID=current \
JWT_ISSUER=https://identity.example.com/ \
JWT_AUDIENCE=your-api \
AUTH_ADMIN_API_KEY='use-a-random-secret-of-at-least-32-bytes' \
AUTH_PASSWORD_PEPPER='use-a-separate-stable-random-secret-of-at-least-32-bytes' \
cargo run -p auth_service
```

Each public key file must be named `<kid>.pem`. New tokens include the active
`kid`; tokens without one use `JWT_LEGACY_KEY_ID`, which defaults to the active
signing ID.

For rotation, first deploy both old and new public keys to `JWT_PUBLIC_KEYS_DIR`
on every verifier. Then switch `JWT_PRIVATE_KEY_PATH` and
`JWT_SIGNING_KEY_ID` to the new key, keeping the old public key available and
setting `JWT_LEGACY_KEY_ID` to the old ID for pre-`kid` tokens. Remove the old
public key only after the maximum token lifetime has elapsed, and then set
`JWT_LEGACY_KEY_ID` to the new ID before restarting.

The PostgreSQL schema is created automatically. `AUTH_ADDR` defaults to
`127.0.0.1:8081`. Tokens expire after 900 seconds by default;
`ACCESS_TOKEN_TTL_SECONDS` accepts values from 60 to 3600. Set
`AUTH_COOKIE_SECURE=false` only for local HTTP development. Implicit flow is
disabled unless `ENABLE_IMPLICIT_FLOW=true`. The in-process verified-JWT cache
holds up to 10,000 entries by default; set `JWT_VERIFY_CACHE_CAPACITY` from 1 to
1,000,000 to change the bound.

## Management API

All management endpoints require `Authorization: Bearer $AUTH_ADMIN_API_KEY`.
Passwords use Argon2id with an independent random salt and the server-side
`AUTH_PASSWORD_PEPPER`; they must contain at least 12 bytes. Keep the pepper
stable and backed up: changing it prevents verification of peppered hashes.
Legacy Argon2 hashes without a pepper are rehashed with the configured pepper
after a successful login.

- `POST /v1/users` creates a user with `{ "email": "...", "password": "..." }`.
- `GET /v1/users` and `GET /v1/users/{id}` list or inspect users.
- `PATCH /v1/users/{id}` updates `email`, `password`, or `active`.
- `DELETE /v1/users/{id}` deactivates the user; it does not delete account data.
- `POST /v1/service-clients` creates a service client. Its generated secret is
  returned once. Optional `scopes` defaults to `api:read`.
- `GET /v1/service-clients` lists clients; `PATCH /v1/service-clients/{id}`
  updates scopes or active status; `DELETE` deactivates the client.
- `POST /v1/public-clients` registers a PKCE client with `client_id`,
  `redirect_uri`, and optional `scopes`. Redirect URIs must use HTTPS, except
  loopback HTTP URLs for local development.

## OAuth

Authorization Code flow requires `response_type=code`, a non-empty `state`, and
`code_challenge_method=S256`. The user signs in and approves the requested
scopes at `GET /oauth/authorize`; the authorization code expires after five
minutes and is single-use. Exchange it at `POST /oauth/token` using form fields
`grant_type=authorization_code`, `client_id`, `redirect_uri`, `code`, and
`code_verifier`.

For service-to-service access, `POST /oauth/token` accepts form fields
`grant_type=client_credentials`, `client_id`, `client_secret`, and optional
space-separated `scope`. The returned token identifies the service client and
contains only scopes granted to that client.

The legacy implicit flow uses `response_type=token` and returns the access token
in the redirect fragment. It is disabled by default and should only be enabled
for legacy clients. New clients should use Authorization Code + PKCE.

`GET /v1/verify` accepts a bearer access token and returns its subject, principal
type (`user` or `service`), and scopes. It verifies signature, issuer, audience,
and expiration, resolving `kid` through the public-key directory (or the legacy
ID for tokens without `kid`). Successful RS256 validations are cached by
SHA-256 token digest
in 16 shards; invalid tokens are never cached, and entries expire at the token's
`exp` claim. The cache is local to one process and is cleared on restart. Since
access tokens are self-contained JWTs, deactivating an account or client does not
revoke already-issued tokens; keep token TTL short. Use a shared introspection or
revocation mechanism if immediate invalidation is required across instances.

## Modules

- `main.rs` wires Axum routes and runtime configuration.
- `management.rs` handles user and service-client lifecycle endpoints.
- `oauth.rs` handles browser authorization and OAuth grants.
- `tokens.rs` issues signed access tokens; `verification.rs` validates them.
- `jwt_cache.rs` caches successful verification results with expiry and capacity
  bounds.
- `models.rs`, `security.rs`, and `state.rs` hold shared types, security helpers,
  and application state; `services/database.rs` owns the PostgreSQL connection,
  schema setup, and persistence queries.

## Tests

Run `cargo test -p auth_service`. Tests live under `tests/` and cover password
hashing and migration, strict PKCE/implicit-flow policy, redirect handling,
template rendering, JWT cache bounds, token claims, and old/new-key overlap.