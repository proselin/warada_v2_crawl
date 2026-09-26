# T009 — Require PostgreSQL for Drizzle

**Status:** done
**Created:** 2026-08-31
**Plan:** [P002-260831-postgres-only-drizzle](../plans/P002-260831-postgres-only-drizzle.md)

## 1. Plan link

Implements P002 by making the configured PostgreSQL host the only database target.

## 2. Impact of work

- `package.json` and `bun.lock` — remove PGlite.
- `src/lib/config.ts`, `src/lib/db.ts`, and `src/index.ts` — remove PGlite selection and schema bootstrapping.
- `src/**/*.test.ts` — keep mutating database tests opt-in for a disposable database.

## 3. Definition of Done

- [x] Drizzle uses only `drizzle-orm/postgres-js`.
- [x] Startup does not issue database DDL.
- [x] Database-mutating tests do not run without explicit opt-in.

## 4. Work history

- 2026-08-31: Removed the embedded PGlite fallback and bootstrap DDL, required `DATABASE_URL`, and guarded database integration tests.
