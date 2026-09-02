# P002 — Require PostgreSQL for Drizzle

**Status:** done
**Created:** 2026-08-31

## 1. Reason

The database host is configured, so the crawler must always use the shared PostgreSQL database rather than an embedded fallback.

## 2. Precondition

`DATABASE_URL` contains a reachable PostgreSQL connection string, and the shared schema continues to be owned by the monolith.

## 3. Post-condition

All Drizzle queries use `postgres.js` with `DATABASE_URL`; the application creates no embedded database or DDL.

## 4. Definition of Done

- [x] PGlite and its bootstrap DDL are removed.
- [x] `DATABASE_URL` is required and validated as a PostgreSQL URL.
- [x] The Postgres-only database client type-checks.

## 5. Logic

Use the existing `drizzle-orm/postgres-js` driver and existing PostgreSQL schema definitions directly. Keep sequence allocation with the configured connection because the shared tables require explicit sequence values.

## 6. Impact

- `package.json` and `bun.lock` — remove the embedded database dependency.
- `src/lib/config.ts` — require a valid PostgreSQL connection URL.
- `src/lib/db.ts` and `src/index.ts` — remove fallback bootstrap behavior.
- Database integration tests — require explicit opt-in against a disposable Postgres database.

## 7. Linked references

- Related complete: [C001-260829-crawl-service-hono-migration](../completes/C001-260829-crawl-service-hono-migration.md)
