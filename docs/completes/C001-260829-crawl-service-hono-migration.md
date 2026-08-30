# C001 — Crawl service Hono migration

**Resolves plan:** [P001-260829-crawl-service-hono-migration](../plans/P001-260829-crawl-service-hono-migration.md)
**Closed:** 2026-08-29

## Tasks included
- [T001-260829-config-and-env-loader](../tasks/T001-260829-config-and-env-loader.md)
- [T002-260829-drizzle-schema-and-db-client](../tasks/T002-260829-drizzle-schema-and-db-client.md)
- [T003-260829-minio-client-wrapper](../tasks/T003-260829-minio-client-wrapper.md)
- [T004-260829-nettruyen-extraction-service](../tasks/T004-260829-nettruyen-extraction-service.md)
- [T005-260829-tag-service-and-enum-codes](../tasks/T005-260829-tag-service-and-enum-codes.md)
- [T006-260829-crawl-orchestration-and-sse-broadcaster](../tasks/T006-260829-crawl-orchestration-and-sse-broadcaster.md)
- [T007-260829-hono-routes-auth-and-bootstrap](../tasks/T007-260829-hono-routes-auth-and-bootstrap.md)
- [T008-260829-standalone-cleanup-script](../tasks/T008-260829-standalone-cleanup-script.md)

## Summary

Built the on-demand Hono/Bun crawl service: typed shared-schema access,
NetTruyen extraction, MinIO two-phase image storage, tag persistence, sequential
chapter progress events, API-key-protected crawl endpoints, and standalone/startup
temp cleanup. The project supports an embedded PGlite database only when
`DATABASE_URL` is absent, enabling local tests and development; configured
PostgreSQL instances remain migration-free.

## Originating backlog

[B001-260829-crawl-service-hono-migration](../backlogs/B001-260829-crawl-service-hono-migration.md)
