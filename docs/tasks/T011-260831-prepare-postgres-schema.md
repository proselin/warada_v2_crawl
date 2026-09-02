# T011 — Prepare PostgreSQL schema

**Status:** done
**Created:** 2026-08-31
**Plan:** [P004-260831-prepare-postgres-schema](../plans/P004-260831-prepare-postgres-schema.md)

## 1. Plan link

Implements P004 by adding and executing explicit PostgreSQL schema preparation.

## 2. Impact of work

- `drizzle/0000_prepare_postgres.sql` — table, sequence, and foreign-key definitions.
- `src/scripts/prepare-db.ts` and `package.json` — `db:prepare` command.

## 3. Definition of Done

- [x] The command targets the configured PostgreSQL database.
- [x] Setup is idempotent and is not run on application startup.
- [x] Required relations are present after execution.

## 4. Work history

- 2026-08-31: Added an explicit idempotent schema preparation command for the configured empty PostgreSQL database.
