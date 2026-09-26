# T012 — Add crawl error logs

**Status:** done
**Created:** 2026-09-01
**Plan:** [P005-260901-crawl-error-logs](../plans/P005-260901-crawl-error-logs.md)

## 1. Plan link

Implements P005 by logging errors at the existing crawler service and HTTP boundaries.

## 2. Impact of work

- `src/index.ts` — log unhandled HTTP request errors.
- `src/routes/crawl.ts` — log rejected invalid requests and absent SSE comics.
- `src/services/crawl.ts` — log crawl and retry failures before they propagate.

## 3. Definition of Done

- [x] Failure logs contain relevant request or crawl identifiers.
- [x] Existing response behavior remains unchanged.
- [x] No credentials or object payloads are logged.

## 4. Work history

- 2026-09-01: Added service and HTTP error-boundary logs for failures that previously escaped without context.
