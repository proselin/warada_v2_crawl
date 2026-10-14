# Infrastructure

The local Docker Compose stack runs the crawler HTTP/gRPC service,
the auth service, the GraphQL display service, the built frontend, and a Caddy
gateway. Only the gateway is published to the host; services communicate over
Compose's private network.

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

Use different random values for `CRAWL_API_KEY`, `AUTH_ADMIN_API_KEY`, and
`AUTH_PASSWORD_PEPPER`. The admin key and password pepper must each be at
least 32 bytes.

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
filesystem storage. The crawler initializes SQLite schema at startup, and the
auth service creates its own tables on the same `sqlite-data` volume.

Each service image builds only its selected Rust crate or UI workspace and
copies only the executable or static assets into the runtime image. Container-
specific `.dockerignore` files keep unrelated source out of each build context;
BuildKit caches Rust and npm dependencies between builds. The activity remote
builds the UI-kit workspace first because it imports that package.

## CI/CD

Run the full build-and-test pipeline locally, without publishing images:

```sh
npm run ci:local
```

This runs project checks and tests, builds all Compose images, then runs E2E
against those images. Docker Compose and Playwright Chromium are required. Use
`E2E_PORT=19080 npm run ci:local` to choose another local port.

Pull requests into `develop` or `main` run checks for affected areas:
Rust changes build and test the Rust workspace; UI changes build and test all
UI workspaces without running Rust checks. Changes to Compose, Containerfiles,
or E2E tests also run the full system E2E suite.

Each push to `develop` runs the full build and test pipeline, builds all six
images, runs full system E2E against them, and publishes them to GHCR only if
the tests pass. A pull request from `develop` to `main` pulls those exact
SHA-tagged images and reruns full system E2E. Merging that pull request
promotes the same images to `main` and `latest` without rebuilding. Wait for
the `develop` push workflow to finish before opening/updating the release pull
request.

Tags include the commit SHA and `develop`. Build outputs, image archives, and
test reports are retained as workflow artifacts for 14 days. The pipeline
publishes images but does not deploy them to servers.

Protect `develop` and `main` with pull-request-only changes and require the
`checks` status check before merge. Configure branch protection in GitHub
repository settings; the workflow cannot enforce those settings by itself.

## Operations

```sh
docker compose --env-file infra/.env -f infra/compose.yaml ps
docker compose --env-file infra/.env -f infra/compose.yaml logs -f gateway crawler
docker compose --env-file infra/.env -f infra/compose.yaml down
```

`down` keeps database and image volumes. Use `down -v` only when you intend to
delete the persisted database and image data. Back up `infra/secrets/jwt` and
the Docker volumes; never commit the `.env` file or private signing key.
