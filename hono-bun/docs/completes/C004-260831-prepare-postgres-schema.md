# C004 — PostgreSQL schema preparation

**Resolves plan:** [P004-260831-prepare-postgres-schema](../plans/P004-260831-prepare-postgres-schema.md)
**Closed:** 2026-08-31

## Tasks included

- [T011-260831-prepare-postgres-schema](../tasks/T011-260831-prepare-postgres-schema.md)

## Summary

Added an explicit `bun run db:prepare` command that creates the crawler’s PostgreSQL tables, sequences, and foreign keys with idempotent SQL. Schema setup remains separate from normal application startup.
