# P004 — Prepare PostgreSQL schema

**Status:** done
**Created:** 2026-08-31

## 1. Reason

The configured PostgreSQL database is empty, so the crawler’s tables and ID sequences must be created before it can persist data.

## 2. Precondition

`DATABASE_URL` targets the empty PostgreSQL database to prepare.

## 3. Post-condition

The target schema has the crawler tables, foreign keys, and ID sequences matching the existing Drizzle definitions.

## 4. Definition of Done

- [x] An explicit idempotent preparation command exists.
- [x] The required tables and sequences are created.
- [x] The schema is verified after preparation.

## 5. Logic

Keep schema creation explicit through `bun run db:prepare`, rather than running DDL whenever the application starts. The SQL creates the existing shared table contract and is safe to re-run.

## 6. Impact

- `drizzle/0000_prepare_postgres.sql` — idempotent PostgreSQL schema setup.
- `src/scripts/prepare-db.ts` and `package.json` — preparation command.

## 7. Linked references

- Related complete: [C002-260831-postgres-only-drizzle](../completes/C002-260831-postgres-only-drizzle.md)
