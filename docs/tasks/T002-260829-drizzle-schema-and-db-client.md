# T002 — Drizzle schema and DB client (no migrations)

**Status:** pending
**Created:** 2026-08-29
**Plan:** [P001-260829-crawl-service-hono-migration](../plans/P001-260829-crawl-service-hono-migration.md)

## 1. Plan link

Implements Section 5.10's `src/lib/db.ts` and `src/lib/schema.ts`, mirroring
the real schema verified in Section 5.2 (including the exact enum wire codes
in 5.2.1). Depends on T001 (config) for the Postgres connection string.

## 2. Impact of work

- `crawl-app/src/lib/schema.ts` — new file: Drizzle table definitions for
  `comics`, `chapters`, `images`, `tags`, `comic_tags`, matching column
  names/types/nullability/uniqueness exactly as in
  `warada_v2/src/main/resources/db/migration/initialize-270925.sql` and
  `add-tags-table-260304.sql` (Plan Section 5.2). `crawling_status` and
  `type` columns are typed as plain string columns in Drizzle — the
  INIT/STARTED/FINISHED ↔ "0000"/"0001"/"9999" and THUMB_IMAGE/CHAPTER_IMAGE
  ↔ "THUMB"/"CHAPTER_IMAGE" conversions happen in application code (see T005),
  not in the schema.
- `crawl-app/src/lib/db.ts` — new file: `postgres` (postgres.js) client +
  Drizzle instance built from `config.DATABASE_URL`. No `drizzle-kit push` or
  `generate` is ever run against the shared DB from this repo.
- `crawl-app/package.json` — add `drizzle-orm`, `postgres` dependencies and
  `drizzle-kit` as a dev dependency (types only).

## 3. Definition of Done

- [ ] `schema.ts` defines all 5 tables with correct PK/FK/unique constraints
      matching the live SQL migrations (cross-checked column-by-column against
      Plan Section 5.2, not re-derived from `ss.md`).
- [ ] `db.ts` exports a working Drizzle instance; a smoke query
      (`SELECT 1` or `select().from(comics).limit(1)`) succeeds against a real
      or locally-tunneled shared Postgres instance.
- [ ] No `drizzle-kit push`/`generate`/migration command is added to
      `package.json` scripts — schema file is for typing only.
- [ ] Confirm zero DDL is issued by the app at any point (Postgres query log
      or `postgres.js` debug output shows only `SELECT`/`INSERT`/`UPDATE`).

## 4. Work history

- 2026-08-29: Task created from plan P001 Section 5.2/5.10, not yet started.
