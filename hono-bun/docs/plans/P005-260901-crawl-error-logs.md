# P005 — Add crawl error logs

**Status:** done
**Created:** 2026-09-01

## 1. Reason

Unhandled crawler failures need enough context to diagnose failed metadata, database, and object-storage operations.

## 2. Precondition

The crawler emits lifecycle trace events through `src/lib/log.ts`.

## 3. Post-condition

Every unhandled crawl failure is logged with crawl context and the corresponding HTTP request failure is logged once before a 500 response.

## 4. Definition of Done

- [x] Top-level crawl and retry failures log context and error messages.
- [x] HTTP 500 responses log method and path.
- [x] Rejected invalid JSON and missing SSE comics are traceable.

## 5. Logic

Add logs only at the existing service and HTTP error boundaries. Keep the chapter-level error log already present, so errors are neither swallowed nor logged at every lower-level operation.

## 6. Impact

- `src/index.ts` — Hono error boundary logging.
- `src/routes/crawl.ts` — rejected request and SSE lookup tracing.
- `src/services/crawl.ts` — crawl and retry failure context.

## 7. Linked references

- Related complete: [C003-260831-crawl-trace-logs](../completes/C003-260831-crawl-trace-logs.md)
