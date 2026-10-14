# System architecture

## Repository shape

```text
.
├── crates/
│   ├── apps/
│   │   ├── auth-service/       Axum OAuth, user/client management, JWT
│   │   ├── comic-crawler/      HTTP API, crawl queue, PostgreSQL, gRPC server
│   │   └── display-service/    GraphQL API backed by crawler gRPC
│   └── libs/
│       └── crawler-proto/      Protobuf schema and generated tonic client/server
├── ui/
│   ├── apps/
│   │   ├── display/            React/Rsbuild federation host
│   │   └── activity-remote/    Independently built activity panel remote
│   └── libs/
│       └── ui-kit/             Shared UI package and federation remote
├── infra/                      Docker Compose, Dockerfiles, Caddy gateway
├── e2e/                        Playwright system tests and disposable stack runner
└── docs/                       Architecture and engineering guidance
```

Cargo workspace members are declared in the root `Cargo.toml`. Moon registers
the root Rust workspace, Rust applications and library, and each frontend
project in `.moon/workspace.yml`; project-specific tasks live in each
project's `moon.yml`. Frontend packages are npm workspaces declared in the
root `package.json`.

## Runtime services

| Component | Responsibility | Internal address / protocol |
| --- | --- | --- |
| PostgreSQL | Persistent catalog, chapters, crawl jobs, users, OAuth clients, and authorization codes | `postgres:5432` |
| Crawler | Crawl queue worker, catalog storage, image storage, protected crawl HTTP API, and crawler gRPC server | HTTP `crawler:3000`, gRPC `crawler:50051` |
| Auth | User and OAuth client management, OAuth grants, RS256 issuance and token verification | HTTP `auth:8081` |
| Display | GraphQL read API that calls the crawler over gRPC | HTTP `display:3001` |
| Display host | Browser dashboard shell and Module Federation host | Static Nginx service |
| Activity remote | Separately built federation component showing crawl jobs | Static Nginx service |
| UI kit | Shared frontend utilities emitted as ESM and as a federation remote | Static Nginx service |
| Caddy gateway | Public HTTP entry point and path-based routing | Host port `8080` by default |

Compose publishes only the gateway. Services and the database stay on the
Compose network. Compose health checks and `depends_on` order startup; they do
not replace application-level handling of dependency failures.

## Request and data flows

### Dashboard catalog and activity

1. The browser loads the static dashboard from Caddy at `/`.
2. The dashboard posts GraphQL queries to `/graphql`; Caddy forwards these to
   the display service.
3. Display calls the crawler's tonic/gRPC interface using types from
   `crates/libs/crawler-proto`.
4. The crawler reads catalog, chapter, and crawl-job records from PostgreSQL.
5. Display maps protobuf records into GraphQL objects and returns JSON.
6. The browser renders catalog and chapter data. The independently built
   activity remote queries crawl jobs through the same GraphQL route.

The GraphQL API is read-only today. Crawl management is exposed separately by
the crawler HTTP API and requires `x-crawl-api-key`.

The current protobuf service is intentionally read-only and exposes
`ListComics`, `GetComic`, `ListChapters`, and `ListCrawlJobs`. Keep the schema
in `crates/libs/crawler-proto/proto/crawler.proto`; Cargo's build script
generates tonic client/server bindings from that file.

### Crawl submission

The crawler's `/api/v1/crawl/nettruyen/comic` and retry endpoints validate the
configured API key and enqueue work in PostgreSQL. The background queue worker
claims pending jobs; progress can be observed through its progress endpoint.
The HTTP service also exposes `/health`. Auth exposes `/health`; display has
`/graphql` and `/playground`.
The E2E tests intentionally submit invalid requests only, so they verify route
and authentication behavior without contacting the external comic source.

### Authentication

Auth owns HTTP OAuth and management endpoints. Management routes require the
configured admin bearer key. Public clients use Authorization Code + PKCE;
service clients use the client-credentials grant. Auth signs short-lived
RS256 JWTs with the private key and verifies them with the public-key directory
at `/v1/verify`. Public verification keys are selected by JWT `kid`, with a
legacy key ID for older tokens. Keep private signing keys and all credentials
outside source control.

JWT verification is local to the auth process and successful verification
results are cached in bounded in-memory shards. Issued tokens are not
immediately revoked when a user/client is deactivated; token lifetime bounds
that exposure.

### Storage

The crawler schema is initialized from
`crates/apps/comic-crawler/migrations/0001_init_database.sql` when PostgreSQL
creates a fresh data volume. Auth creates its own tables at application
startup. Both services currently receive the same PostgreSQL database URL.
Crawler image files are stored in the `crawler-images` Docker volume; MinIO is
not part of the current implementation.

The migration mount is an initialization hook, not a migration runner for an
existing database volume. Schema changes on deployed volumes need an explicit
migration and rollout strategy.

## Frontend and Module Federation

`ui/apps/display` is the host. It consumes:

- `activity/Activity` from `ui/apps/activity-remote`.
- `ui-kit/status-class` from `ui/libs/ui-kit`.

The host and activity remote use Rsbuild/Rspack with the Module Federation
plugin. UI kit uses Rslib to produce an ESM package and an MF remote. React and
React DOM are configured as singletons. TypeScript source and build configs use
the native TypeScript Go compiler (`tsgo`); Rstest tests the UI kit with the
Rslib adapter.

In Compose production, the host's remote manifest URLs are injected at build
time using `PUBLIC_BASE_URL`. Caddy serves the remotes on the same origin under
`/remotes/activity/` and `/remotes/ui-kit/`. Keep the federation public paths,
host manifest URLs, Caddy routes, and static asset paths consistent when
changing deployment routing.

Frontend builds are deliberately sequential:

```text
ui-kit -> activity-remote -> display-host
```

The host/remote builds can consume UI-kit output; do not parallelize tasks that
write to the same UI-kit build output.

## Gateway routes

The routing source of truth is `infra/Caddyfile`:

| Public path | Upstream | Path handling |
| --- | --- | --- |
| `/` and unmatched paths | `frontend:80` | Preserved |
| `/graphql`, `/playground` | `display:3001` | Preserved |
| `/oauth/*`, `/v1/*` | `auth:8081` | Preserved |
| `/api/*`, `/health` | `crawler:3000` | Preserved |
| `/tools`, `/tools/*` | `TOOLS_BACKEND_URL` | `/tools` prefix stripped |
| `/remotes/activity/*` | `activity:80` | Prefix stripped |
| `/remotes/ui-kit/*` | `ui-kit:80` | Prefix stripped |

The gateway is configuration-driven; service-specific application logic
belongs in the services, not in Caddy.

## Architecture constraints

- Keep deployable Rust services in `crates/apps/` and shared Rust code in
  `crates/libs/`.
- Keep deployable frontend apps in `ui/apps/` and shared frontend packages in
  `ui/libs/`.
- Add a UI library only when it is shared or has a concrete independent build
  or test need; avoid speculative package splitting.
- Protobuf types belong in `crawler-proto`; do not hand-maintain duplicate
  network contract structs at both ends.
- Keep the crawler as the owner of crawling and crawler data. Display remains
  a read-oriented GraphQL adapter rather than another database client.
- Update this document when service ownership, protocols, storage, routing,
  federation, or workspace layout changes.
