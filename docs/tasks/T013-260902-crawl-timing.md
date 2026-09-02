# T013 — Add crawl timing

**Status:** done
**Created:** 2026-09-02
**Plan:** [P006-260902-crawl-timing](../plans/P006-260902-crawl-timing.md)

## 1. Plan link

Implements P006 by adding native duration measurements to trace events.

## 2. Impact of work

- `src/lib/log.ts` — calculate elapsed milliseconds.
- `src/index.ts`, `src/lib/db.ts`, and `src/services/crawl.ts` — report timing fields.

## 3. Definition of Done

- [x] Timing requires no dependency or external service.
- [x] HTTP, database-ID, crawl, retry, and chapter durations are logged.
- [x] Existing behavior remains unchanged.

## 4. Work history

- 2026-09-02: Added `performance.now()`-based `durationMs` measurements at request and crawler lifecycle boundaries.
