# T004 — NetTruyen scraping/extraction service

**Status:** pending
**Created:** 2026-08-29
**Plan:** [P001-260829-crawl-service-hono-migration](../plans/P001-260829-crawl-service-hono-migration.md)

## 1. Plan link

Implements Section 5.10's `src/lib/nettruyen-client.ts` and
`src/services/extractor.ts`, porting the exact scraping logic verified in
Plan Sections 5.3 (regexes, JSON-LD parsing, chapter list shape) and 5.5
(image download HTTP headers). Depends on T001 (config) for `NETTRUYEN_URL`
and T003 (MinIO) for saving downloaded images to `temp/`.

## 2. Impact of work

- `crawl-app/src/lib/nettruyen-client.ts` — new file: `fetch` wrappers for
  `GET {NETTRUYEN_URL}/truyen-tranh/{slugNId}` (HTML) and
  `GET {NETTRUYEN_URL}/Comic/Services/ComicService.asmx/ChapterList?slug=&comicId=`
  (JSON), typed per Plan 5.3 step 2's field list.
- `crawl-app/src/services/extractor.ts` — new file:
  - `extractComicDetail(slugNId)`: fetch comic HTML, regex-extract `slug`
    (`gOpts.comicSlug`) and `comicId` (`gOpts.comicId`), parse JSON-LD
    `<script type="application/ld+json">` blocks (flatten `@graph`, find
    `@type` including `"ComicSeries"`), fetch chapter list, compute each
    chapter's `originPathParams`/`originUrl` (Plan 5.3 step 2), download the
    thumbnail (Plan 5.3 step 3).
  - `extractChapterDetail(originPathParams)`: fetch chapter HTML, regex-match
    `data-sv1`/`data-sv2` pairs in document order, download each image
    concurrently trying `sv1` then falling back to `sv2` per slot (Plan 5.3
    step 4).
  - Image download helper implementing the exact header set from Plan 5.5
    (`Origin`, `Referer`, `Accept: */*`, `Content-Type:
    application/octet-stream`, CORS/sec-fetch headers), non-200 → throw,
    saves to MinIO temp path via T003, filename pattern
    `{yyyyMMdd_HHmmss}_{uniqueName}{ext}`.

## 3. Definition of Done

- [ ] Comic HTML fixture (captured from a real NetTruyen comic page) parses
      to the expected slug/comicId/title/author/genre/description via a unit
      test — no network call needed for this test.
- [ ] Chapter-list JSON fixture parses to the expected `originPathParams`/
      `originUrl` per chapter.
- [ ] Chapter-detail HTML fixture with known `data-sv1`/`data-sv2` pairs
      extracts image URLs in correct page-position order.
- [ ] A live (manual, not CI) smoke test against `nettruyenar.com` for one
      real comic slug confirms end-to-end extraction + thumbnail download
      succeeds.
- [ ] Every outbound image request sets the exact header set from Plan 5.5 —
      verified via a mocked `fetch` capturing request headers in a unit test.

## 4. Work history

- 2026-08-29: Task created from plan P001 Section 5.3/5.5/5.10, not yet started.
