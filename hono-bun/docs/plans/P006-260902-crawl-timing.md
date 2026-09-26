# P006 — Add crawl timing

**Status:** done
**Created:** 2026-09-02

## 1. Reason

Operators need request and crawler duration measurements to identify slow operations.

## 2. Precondition

The application already emits dependency-free trace events.

## 3. Post-condition

Trace events report millisecond durations for HTTP requests, database ID allocation, crawl and retry runs, chapter batches, and individual chapters.

## 4. Definition of Done

- [x] HTTP requests emit a duration.
- [x] Crawl and retry outcomes emit a duration.
- [x] Chapter and sequence allocation timings are traceable.

## 5. Logic

Use Bun’s native `performance.now()` through the existing trace helper. Emit durations only at existing lifecycle boundaries; do not add a metrics service or dependency.

## 6. Impact

- `src/lib/log.ts` — elapsed-time helper.
- `src/index.ts`, `src/lib/db.ts`, and `src/services/crawl.ts` — duration fields in trace events.

## 7. Linked references

- Related complete: [C005-260901-crawl-error-logs](../completes/C005-260901-crawl-error-logs.md)
