# P003 — Add crawl trace logs

**Status:** done
**Created:** 2026-08-31

## 1. Reason

Crawl progress needs concise server-side tracing for requests, persistence, and chapter failures.

## 2. Precondition

The crawler runs as a Hono application and uses the configured PostgreSQL database.

## 3. Post-condition

Operators can follow request, crawl, retry, chapter, and ID-allocation lifecycle events without exposing credentials or downloaded content.

## 4. Definition of Done

- [x] HTTP requests are logged.
- [x] Crawl and retry lifecycle transitions are logged.
- [x] Chapter failures include an error message and identifiers.

## 5. Logic

Use a small local `console` wrapper for consistent trace events and Hono's built-in request logger. Do not add a logging dependency.

## 6. Impact

- `src/lib/log.ts` — trace event helpers.
- `src/index.ts`, `src/routes/crawl.ts`, `src/lib/db.ts`, and `src/services/crawl.ts` — lifecycle logs.

## 7. Linked references

- Related complete: [C002-260831-postgres-only-drizzle](../completes/C002-260831-postgres-only-drizzle.md)
