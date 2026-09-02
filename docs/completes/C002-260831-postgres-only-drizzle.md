# C002 — PostgreSQL-only Drizzle

**Resolves plan:** [P002-260831-postgres-only-drizzle](../plans/P002-260831-postgres-only-drizzle.md)
**Closed:** 2026-08-31

## Tasks included

- [T009-260831-postgres-only-drizzle](../tasks/T009-260831-postgres-only-drizzle.md)

## Summary

Removed the embedded PGlite fallback and bootstrap schema DDL. Drizzle now connects only through `postgres.js` using a required PostgreSQL `DATABASE_URL`; integration tests that mutate database data require explicit opt-in.
