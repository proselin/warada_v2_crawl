# T006 — Crawl orchestration service and SSE broadcaster

**Status:** done
**Created:** 2026-08-29
**Plan:** [P001-260829-crawl-service-hono-migration](../plans/P001-260829-crawl-service-hono-migration.md)

## 1. Plan link

Implements Section 5.10's `src/services/crawl.ts` and `src/lib/broadcaster.ts`,
porting `CrawlServiceImpl`'s orchestration and transaction boundaries (Plan
Section 5.6) plus the active-crawl guard fix from Section 5.7 (which the real
Quarkus code declares but never wires up). Depends on T002 (DB), T003 (MinIO),
T004 (extractor), T005 (tags/enums).

## 2. Impact of work

- `crawl-app/src/lib/broadcaster.ts` — new file:
  `Map<string, Set<SSEStreamingApi>>` for subscribe/emit/cleanup-on-disconnect
  (mirrors `CrawlProgressBroadcasterImpl`), plus a `Set<string>` of
  `activeComics` with `markActive`/`isActive`/`markInactive` — **and this
  time actually call them** from `crawl.ts` (Plan 5.7 fix #2).
- `crawl-app/src/services/crawl.ts` — new file:
  - `crawlNettruyenComic(slugNId)`: early-exit 409-equivalent (throw a typed
    `ConflictError`) if `origin_path_params` already exists (Plan 5.6 step 1);
    mark comic active; extract comic detail (T004); rename thumbnail to
    permanent path (T003); resolve tags (T005); insert `comics` + thumbnail
    `images` + all `chapters` rows in one DB transaction
    (`crawling_status=FINISHED` on the comic row per Plan 5.6 step 2's
    documented quirk, `crawling_status=INIT`/`"0000"` on each chapter);
    increment tag counts; then loop chapters sequentially, emitting
    `processing`/`completed`/`failed` SSE events per Plan 5.6 step 3,
    continuing past individual chapter failures; mark comic inactive when done.
  - `retryFailedChapters(comicSlug)`: throw 404-equivalent if comic not
    found (Plan 5.7 fix #1); throw 409-equivalent if `isActive(comicSlug)`
    (Plan 5.7 fix #2); otherwise re-run the per-chapter logic for every
    chapter with `crawling_status = "0000"` (INIT).
  - Chapter crawl helper: extract chapter detail (T004), rename each image to
    `{comicSlug}/chapters/{chapterName}/{filename}` (T003), insert `images`
    rows + flip chapter to FINISHED in one transaction.

## 3. Definition of Done

- [ ] Crawling a known-duplicate `slugNId` throws before any network call
      (verified via a mock extractor that would fail the test if invoked).
- [ ] A full crawl of a fixture-backed comic (mocked extractor + MinIO)
      persists the comic row (`crawling_status` code `"9999"`), all chapter
      rows (`"0000"` initially), thumbnail image row, and emits the expected
      SSE event sequence for each chapter.
- [ ] A chapter whose extraction/download throws is marked failed (SSE
      `failed` event emitted) and does **not** abort remaining chapters.
- [ ] Calling `retryFailedChapters` for an unknown slug and for a slug with
      an active crawl produce the two distinct error conditions from Plan 5.7.
- [ ] Retry only re-processes chapters still at `"0000"`; already-`"9999"`
      chapters are left untouched.

## 4. Work history

- 2026-08-29: Task created from plan P001 Section 5.6/5.7/5.10, not yet started.
- 2026-08-29: Added crawl orchestration, progress broadcasting, active-crawl
  tracking, and duplicate/not-found/active guard coverage.
