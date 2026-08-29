# T007 — Hono routes, auth middleware, and app bootstrap

**Status:** pending
**Created:** 2026-08-29
**Plan:** [P001-260829-crawl-service-hono-migration](../plans/P001-260829-crawl-service-hono-migration.md)

## 1. Plan link

Implements Section 5.10's `src/routes/crawl.ts` and updates `src/index.ts`,
wiring the REST surface from Plan Section 5.1, the auth decision from Section
5.11, and the startup cleanup pass from Section 5.8. Depends on T001–T006.

## 2. Impact of work

- `crawl-app/src/routes/crawl.ts` — new file:
  - `POST /api/v1/crawl/nettruyen/comic` — parse `{"slug-and-id": string}`,
    call `crawlNettruyenComic`, return `200 "Success"` or map `ConflictError`
    → `409`.
  - `POST /api/v1/crawl/nettruyen/comic/:slug/retry` — call
    `retryFailedChapters`, return `200 "Retry queued"`, map not-found → `404`,
    active-crawl → `409` (Plan 5.7).
  - `GET /api/v1/crawl/progress/:comicSlug` — look up comic first (404 if
    absent, Plan 5.7 fix #3), otherwise `streamSSE` subscribing via the
    broadcaster (T006).
- `crawl-app/src/lib/auth.ts` — new file: Hono middleware checking
  `X-Crawl-Api-Key` header against `config.CRAWL_API_KEY` (T001), `401` on
  mismatch/missing, applied to all three crawl routes only (Plan 5.11).
- `crawl-app/src/index.ts` — updated: register `/health` route (`200 OK`
  plain text/JSON), register the crawl router, run one cleanup pass (T008's
  logic) at startup before accepting requests.

## 3. Definition of Done

- [ ] All three routes return the exact status codes/bodies specified in
      Plan Section 5.1 and the 5.7 fixes, verified via integration-style
      request tests (real or mocked DB/MinIO).
- [ ] Requests missing or with a wrong `X-Crawl-Api-Key` get `401` on all
      three crawl routes; `/health` remains unauthenticated.
- [ ] `GET /health` returns `200` without touching DB/MinIO (cheap liveness
      check only).
- [ ] `bun run dev` starts the server, runs the startup cleanup pass (visible
      in logs), and begins listening without error given a valid `.env`.
- [ ] An SSE client connected to `/progress/{slug}` receives events emitted
      by a concurrently-running crawl for that slug end-to-end.

## 4. Work history

- 2026-08-29: Task created from plan P001 Section 5.1/5.8/5.10/5.11, not yet started.
