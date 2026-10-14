# Infrastructure

The local Docker Compose stack runs PostgreSQL, the crawler HTTP/gRPC service,
the auth service, the GraphQL display service, the built frontend, and a Caddy
gateway. Only the gateway is published to the host; services and PostgreSQL
communicate over Compose's private network.

## Start

From the repository root:

```sh
cp infra/.env.example infra/.env
# Set unique secrets in infra/.env. A convenient secret source is:
openssl rand -hex 32

mkdir -p infra/secrets/jwt/public
openssl genrsa -out infra/secrets/jwt/private.pem 3072
openssl rsa -in infra/secrets/jwt/private.pem -pubout \
  -out infra/secrets/jwt/public/current.pem
chmod 600 infra/secrets/jwt/private.pem

docker compose --env-file infra/.env -f infra/compose.yaml up --build -d
```

Use different random values for `POSTGRES_PASSWORD`, `CRAWL_API_KEY`,
`AUTH_ADMIN_API_KEY`, and `AUTH_PASSWORD_PEPPER`. The admin key and password
pepper must each be at least 32 bytes. `POSTGRES_PASSWORD` should be
hexadecimal (as produced by `openssl rand -hex`) so it is safe in the generated
PostgreSQL URL.

The gateway listens on `http://localhost:8080` (change `GATEWAY_PORT` to
publish another host port). It routes `/` to the frontend, `/graphql` and
`/playground` to display, `/oauth/*` and `/v1/*` to auth, and `/api/*` to the
crawler. `/tools` and `/tools/*` are forwarded to `TOOLS_BACKEND_URL` after
removing the `/tools` prefix; the example points this route to the crawler.
The dashboard is an Rsbuild federation host; the activity panel and shared UI
kit are separate remotes served under `/remotes/activity` and `/remotes/ui-kit`.
Set `PUBLIC_BASE_URL` to the public gateway origin before building for deployment.

The crawler stores images in the persistent `crawler-images` volume. MinIO is
not included because the current crawler implementation writes to local
filesystem storage. PostgreSQL initializes the crawler schema on first
creation of its data volume; the auth service creates its own tables at startup.

## Operations

```sh
docker compose --env-file infra/.env -f infra/compose.yaml ps
docker compose --env-file infra/.env -f infra/compose.yaml logs -f gateway crawler
docker compose --env-file infra/.env -f infra/compose.yaml down
```

`down` keeps database and image volumes. Use `down -v` only when you intend to
delete the persisted database and image data. Back up `infra/secrets/jwt` and
the Docker volumes; never commit the `.env` file or private signing key.
