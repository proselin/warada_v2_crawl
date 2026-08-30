# P001 — Migrate crawl subsystem from warada_v2 (Quarkus) into crawl-app (Hono/Bun)

**Status:** done
**Created:** 2026-08-29
**Linked backlog:** [B001-260829-crawl-service-hono-migration](../backlogs/B001-260829-crawl-service-hono-migration.md)

## 1. Reason

`warada_v2`'s crawl subsystem (NetTruyen scraping → image download → MinIO →
Postgres → SSE progress) currently lives inside the Quarkus monolith at
`com.github.warada_v2.crawl.*`. A prior requirements/design doc (`ss.md`, root
of this repo) specced extracting it into a **second Quarkus** service. The
owner instead wants it rebuilt in **this** repo (`crawl-app`) on **Hono +
Bun**, because crawling is an on-demand, occasional action — not a service
that should run continuously like the monolith. Bun's fast startup and
Hono's minimal footprint make "run it only when I need to crawl" the natural
fit, instead of a JVM microservice with native-image builds, OTel, and
always-on schedulers.

> **Note on sources:** this plan was revised after reading the actual live
> source in `warada_v2` (sibling checkout at
> `/Users/hungnq18/Documents/Projects/github/warada_v2`) — real Liquibase-run
> SQL migrations, real JPA entities, real converters, and the real
> `NettruyenExtractorImpl` / `CrawlServiceImpl` / `CrawlProgressBroadcasterImpl`
> / `FileTempClearer` / `CrawlRestResource` classes — not just `ss.md`'s prose
> summary. Section 5 documents several places where `ss.md`'s described
> behavior (404/409 handling, enum string values) **does not match** what the
> Quarkus code actually does today; those are called out explicitly so the
> Hono port doesn't silently inherit wrong assumptions.
>
> A previous, abandoned attempt at this exact migration exists at
> `/Users/hungnq18/Documents/Projects/github/warada_v2_crawl` — a Rust
> (Axum + Diesel) skeleton with no real crawl logic implemented (placeholder
> handlers only). It contributes no reusable design beyond confirming the
> owner had already settled on "small, on-demand service" before choosing
> Bun/Hono as the final stack.

## 2. Precondition

- `crawl-app` currently has only a bare Hono skeleton (`src/index.ts` returns
  `Hello Hono!`); no crawl logic, DB client, or MinIO client exists yet.
- `warada_v2`'s Postgres schema (`comics`, `chapters`, `images`, `tags`,
  `comic_tags`) continues to be owned and migrated exclusively by the
  monolith (`src/main/resources/db/migration/*.sql`, applied via
  Liquibase-style changelog `db-changelog-master.yaml`). This plan does not
  change that ownership — `crawl-app` must read/write the same schema
  without ever issuing DDL.
- The same MinIO bucket (`warada.image-bucket`, default `warada-images`, set
  in `warada_v2/src/main/resources/application.properties`) is shared;
  `crawl-app` writes objects there, the monolith only ever reads `file_path`.
- Network access to `nettruyenar.com`, the shared Postgres instance, and the
  shared MinIO instance must be reachable from wherever `crawl-app` is run.

## 3. Post-condition

- `crawl-app` can, when started on demand (`bun run dev` / `bun run start`),
  expose the three crawl endpoints (start crawl, retry, SSE progress),
  scrape NetTruyen, download images into MinIO under the same object layout
  the monolith expects, and persist `comics`/`chapters`/`images`/`tags` rows
  into the same shared Postgres schema — all without running any migration.
- `warada_v2` requires **zero code changes**; it keeps reading `images.file_path`
  and other crawled data exactly as today.
- The crawl subsystem can be stopped when not in use; no scheduler, health
  check, or process needs to run continuously for the system to remain
  correct. Temp-object cleanup is designed to work even though the process
  is not resident 24/7 (see 5.6).

## 4. Definition of Done

- [x] Real Postgres schema, entities, enums, converters, repositories read
      directly from `warada_v2` source (not inferred from `ss.md`) — done in
      this planning pass; captured in Section 5.2.
- [x] Real crawl pipeline logic (scraping regexes, HTTP headers, MinIO
      two-phase paths, transaction boundaries, tag resolution) read directly
      from `NettruyenExtractorImpl`/`CrawlServiceImpl`/`TagServiceImpl` —
      captured in Section 5.3–5.5.
- [x] Discrepancies between `ss.md`'s described behavior and the actual
      shipped Quarkus behavior identified and flagged for an owner decision —
      Section 5.7.
- [x] Deployment/portability target and auth strategy decided (Section 5.11)
      — defaulted per the plan's own recommendations since the owner was
      unavailable to confirm; revisit if either default proves wrong once
      real usage patterns are observed.
- [x] This plan is broken into `docs/tasks/T00X-...` files (one task per
      logical implementation slice), each linked back to this plan per
      `docs/docs-rule.md` — see `docs/tasks/T001`–`T008`.

## 5. Logic

### 5.1 REST surface to replicate

| Method | Path | Body / Params | Response |
|---|---|---|---|
| `POST` | `/api/v1/crawl/nettruyen/comic` | `{"slug-and-id": "<slug-n-id>"}` | `200 "Success"` on completion; `409 Conflict` if `originPathParams` (= the same `slug-and-id` value) already exists on a comic |
| `POST` | `/api/v1/crawl/nettruyen/comic/{slug}/retry` | path param `slug` | `200 "Retry queued"` — re-crawls all chapters whose `crawling_status = INIT` for that comic slug |
| `GET` | `/api/v1/crawl/progress/{comicSlug}` | path param, `Accept: text/event-stream` | SSE stream of `CrawlProgressEvent` JSON objects |

`CrawlProgressEvent` shape (from `crawl/models/CrawlProgressEvent.java`):
```ts
type CrawlProgressEvent = {
  type: "processing" | "completed" | "failed" | "skipped"; // enum serializes via its string value
  message: string;
  data: {
    chapterId: number;
    chapterNumber: string;      // e.g. "Chapter 10" (the extracted chapter_name)
    status: "INIT" | "STARTED" | "FINISHED" | null; // CrawlStatus, currently always null in emitted events
    comicSlug: string;
  };
};
```
`SKIPPED` is a declared enum value but is never emitted anywhere in
`CrawlServiceImpl` today — only `processing`/`completed`/`failed` are actually
sent. Keep `skipped` in the type for forward-compat but don't invent new call
sites emitting it unless a real skip case is added.

### 5.2 Real database schema (verified against live SQL migrations)

Source: `warada_v2/src/main/resources/db/migration/initialize-270925.sql`,
`add-tags-table-260304.sql`, `remove-unique-260226.sql`.

```sql
-- sequences: chapter_id_seq, comic_id_seq, image_id_seq, tag_id_seq (all allocationSize=1 in JPA)

comics (
  id bigint PK,                       -- from comic_id_seq
  slug varchar(200) UNIQUE NOT NULL,
  title varchar(255),
  author varchar(255),
  description text,
  status varchar(255) NOT NULL,       -- free-text, e.g. "OnGoing" (set literally by CrawlServiceImpl)
  chapter_count integer,
  crawling_status varchar(255) NOT NULL,  -- stores CODE not name: "0000"|"0001"|"9999", see 5.2.1
  origin_id varchar(255) UNIQUE,      -- NetTruyen's numeric comicId, as string
  origin_url varchar(500) UNIQUE,
  origin_path_params varchar(500),    -- the `slug-and-id` value used for dedup (existsByOriginPathParams)
  thumb_image_id bigint UNIQUE FK -> images.id (ON DELETE SET NULL),
  created_at timestamp(6), updated_at timestamp(6)
)

chapters (
  id bigint PK,                       -- from chapter_id_seq
  comic_id bigint FK -> comics.id (ON DELETE CASCADE),
  chapter_num varchar(255),           -- despite the name, holds the *extracted chapter name/label* (e.g. "Chapter 10"), NOT a number
  position integer,                   -- the numeric `chapter_num` from NetTruyen's API (confusingly named the other way vs. the column)
  crawling_status varchar(255),       -- "0000"|"0001"|"9999"
  type varchar(255),                  -- currently never set by the crawl pipeline (always null)
  origin_url varchar(500) UNIQUE,
  origin_path_params varchar(500) UNIQUE,  -- `${comicSlug}/${chapterSlug}/${chapterOriginId}`
  created_at timestamp(6), updated_at timestamp(6)
)

images (
  id bigint PK,                       -- from image_id_seq
  chapter_id bigint FK -> chapters.id (ON DELETE CASCADE, nullable — thumb images have no chapter_id),
  position integer,
  file_name varchar(1000),
  file_path varchar(1000),            -- permanent MinIO object key, see 5.4
  origin_url varchar(500) UNIQUE,     -- the *pulled* CDN image URL, not the NetTruyen page URL
  type varchar(255),                  -- "THUMB" | "CHAPTER_IMAGE", see 5.2.1
  created_at timestamp(6), updated_at timestamp(6)
)
-- note: images.file_name / images.file_path UNIQUE constraints were DROPPED by remove-unique-260226.sql

tags (
  id bigint PK,                       -- from tag_id_seq
  name varchar(255) NOT NULL,             -- original casing, first-seen display form
  normalized_name varchar(255) UNIQUE NOT NULL,  -- trim().toLowerCase(), dedup key
  comic_count integer NOT NULL DEFAULT 0,         -- incremented every time a comic using this tag is crawled (no decrement path)
  created_at timestamp(6), updated_at timestamp(6)
)

comic_tags (comic_id, tag_id)  -- PK(comic_id, tag_id), both FK ON DELETE CASCADE
```

#### 5.2.1 Enum wire values — **do not use `ss.md`'s assumed values**

`ss.md` describes `CrawlStatus` as `INIT`/`FINISHED` strings and `ImageType`
as `THUMB_IMAGE`/`CHAPTER_IMAGE`. The **actual stored column values** (from
`CrawlStatusConverter`/`ImageTypeConverter` + the enums) are:

| Enum | TS/logical name | **DB column value** |
|---|---|---|
| `CrawlStatus` | `INIT` | `"0000"` |
| `CrawlStatus` | `STARTED` | `"0001"` (declared, never written by the crawl pipeline) |
| `CrawlStatus` | `FINISHED` | `"9999"` |
| `ImageType` | `THUMB_IMAGE` | `"THUMB"` |
| `ImageType` | `CHAPTER_IMAGE` | `"CHAPTER_IMAGE"` |

`crawl-app`'s Drizzle/SQL layer must write these exact code strings into
`crawling_status` / `type` columns, or the monolith's Hibernate converters
will throw `IllegalArgumentException: Unknown CrawlStatus code` when reading
rows written by the Hono service.

### 5.3 NetTruyen scraping (verified against `NettruyenExtractorImpl`)

1. **Comic page**: `GET {NETTRUYEN_URL}/truyen-tranh/{slugNId}` → HTML.
   - Extract `slug` via regex `gOpts\.comicSlug\s*=\s*(['"])(.*?)\1`.
   - Extract numeric `comicId` via regex `gOpts\.comicId\s*=\s*(['"])?(\d+)\1?`.
   - Extract every `<script type="application/ld+json">...</script>` block;
     for each, flatten `@graph` arrays (or treat the root as one node), find
     the node whose `@type` includes `"ComicSeries"`, and map its fields:
     `name`, `alternateName`, `url`, `image`, `author.name`, `genre[]`,
     `description`, `datePublished`, `dateModified`.
2. **Chapter list**: `GET {NETTRUYEN_URL}/Comic/Services/ComicService.asmx/ChapterList?slug={slug}&comicId={comicId}`
   → JSON `{ "data": [ { comic_id, chapter_id, chapter_name, chapter_slug,
   updated_at, chapter_num, data_cdn, webp, reported_at, cdn_sv, image_type,
   image_num } ] }`. For each item compute:
   - `originPathParams = "{slug}/{chapter_slug}/{chapter_id}"`
   - `originUrl = "{NETTRUYEN_URL}/truyen-tranh/{originPathParams}"`
3. **Thumbnail**: download `comic.image` URL via the image-download routine
   (5.5) with `uniqueName = "thumbnail"`.
4. **Chapter detail** (per chapter, on-demand — not during the initial comic
   crawl): `GET {originUrl}` → HTML. Regex-match every
   `data-sv1='...' ... data-sv2='...'` pair (two candidate CDN URLs per image
   slot, in document order = page position). For each pair, attempt
   `sv1` first; if that download fails, fall back to `sv2` for the same slot.
   Downloads for different slots run concurrently.

### 5.4 MinIO object layout (two-phase, verified against `CrawlServiceImpl`)

```
warada-images/                                  ← IMAGE_BUCKET
├── temp/{yyyyMMdd_HHmmss}_{uniqueName}{ext}     ← IMAGE_BUCKET_TEMP_PATH, transient
└── {comicSlug}/
    ├── thumb/{filename}                        ← permanent thumbnail (uniqueName="thumbnail")
    └── chapters/{chapterName}/{filename}        ← permanent chapter image; "chapterName" is
                                                     the extracted label (chapters.chapter_num
                                                     column), NOT the numeric chapter_num/position
```
Every image is `PutObject`'d to `temp/{filename}` at download time, then
`CopyObject` + `RemoveObject` to the permanent path once its owning
comic/chapter DB row is about to be persisted (rename-on-success pattern —
never DB-persist a row pointing at a `temp/` path).

Filename pattern: `{timestamp}_{uniqueName}{extension}` where `timestamp` is
`yyyyMMdd_HHmmss`, `uniqueName` is `"thumbnail"` for the cover or the image's
0-based page `position` for chapter images, and `extension` is derived from
the source URL (e.g. `.jpg`, `.webp`).

### 5.5 Image download HTTP contract (verified against `pullImage`)

Every image GET request must set:
```
Origin: {NETTRUYEN_URL}
Referer: {NETTRUYEN_URL}
Accept: */*
Content-Type: application/octet-stream   (yes, on the request, not just response)
Access-Control-Allow-Origin: *
sec-fetch-mode: cors
sec-fetch-dest: empty
sec-fetch-site: cross-site
```
A non-`200` response throws and the caller marks that image/chapter as
failed. `Content-Type` of the *response* determines the object's stored
content-type in MinIO (default `application/octet-stream` if absent).

### 5.6 Persistence & transaction boundaries (verified against `CrawlServiceImpl`)

1. **Dedup check**: if `comics.origin_path_params = slugNId` already exists →
   409 Conflict, abort before any scraping happens... actually scraping
   happens first in the real code (`extractComicDetail` runs, then the
   duplicate check re-runs inside `persistComic`) — but the **first** dedup
   check (`comicRepository.existsByOriginPathParams`) does run before
   scraping starts, so no network calls occur for a known duplicate. Keep
   this early-exit in the Bun port for the same reason (avoid wasted scraping
   + image downloads).
2. **Transaction 1** (comic + chapters, all-or-nothing): resolve-or-create
   tags (`resolveOrCreateTags`), build the `Comic` row
   (`crawling_status = FINISHED` **immediately**, even though its chapters
   start at `INIT` — this is the real, if slightly inconsistent, behavior:
   "comic-level" `crawling_status` only ever reflects that comic *metadata*
   crawl succeeded, not that all chapters are done), build the thumbnail
   `Image` row, build one `Chapter` row per extracted chapter
   (`crawling_status = INIT`), persist all of it together, then increment
   each involved tag's `comic_count`.
3. **Per-chapter loop** (sequential, one broadcaster `processing` event
   before, one `completed`/`failed` event after): for each chapter stub,
   scrape its detail page (5.3 step 4), download all page images to
   `temp/`, rename each to its permanent path, then in **its own
   transaction** insert the `Image` rows and flip that chapter's
   `crawling_status` to `FINISHED`. If any exception is thrown during
   scraping/download/persist for a chapter, log it, emit a `failed` SSE
   event with the exception message, and **continue to the next chapter**
   — one chapter failing never aborts the whole comic crawl.
4. **Retry**: re-run step 3's per-chapter logic for every chapter of the
   given comic slug whose `crawling_status = INIT` (i.e., never finished).

### 5.7 Discrepancies between `ss.md` and the real Quarkus code — decisions needed

`ss.md` (the earlier requirements doc) describes some behavior the actual
current Java code does **not** implement. Recommendation: **fix these in the
Bun port** rather than reproduce the gaps, since they're cheap to do right
and were clearly the intended design.

| `ss.md` says | Real code today | Recommendation for `crawl-app` |
|---|---|---|
| `POST retry` on unknown slug → `404` | `CrawlServiceImpl.retryFailedChapters` just logs a warning and returns; the REST resource still replies `200 "Retry queued"` regardless | Implement the `404` — look up the comic first, return 404 if absent |
| `POST retry` on comic with an active crawl → `409` | `CrawlProgressBroadcaster.markActive/isActive/markInactive` exist but are **never called** anywhere in `CrawlServiceImpl` — there is no real "active crawl" guard today | Implement it: call `markActive`/`markInactive` around the crawl pipeline in `crawlNettruyenComic`, and check `isActive` at the top of both the comic-crawl and retry handlers, returning `409` if already active |
| `GET progress/{slug}` on unknown slug → `404` | `CrawlRestResource.subscribeToCrawlProgress` subscribes unconditionally, no existence check | Implement the `404` — look up the comic first |
| `CrawlStatus` stored as `INIT`/`FINISHED` strings | Stored as codes `"0000"`/`"0001"`/`"9999"` (5.2.1) | Use the real codes — this one is not optional, it's a wire-format compatibility requirement, not a nice-to-have |

### 5.8 On-demand cleanup strategy (replaces `quarkus-scheduler` cron)

No always-on cron. Instead:
- Run one cleanup pass automatically at process startup (covers "start the
  app, it tidies up stale temp objects from the last session, then serves").
- Provide `bun run cleanup` as a standalone script doing the same scan+delete
  logic, so the *host* can wire it into `cron`/`systemd timer` independently
  of whether the Hono server is running.
- Cleanup logic itself (list `temp/` prefix recursively, delete objects with
  `lastModified < now - CLEANUP_MIN_AGE_MINUTES`, skip entirely if
  `CLEANUP_ENABLED=false`) is a direct port of `FileTempClearer.cleanTempObjects`.

### 5.9 Architecture translation: Quarkus concept → Hono/Bun equivalent

| Concern | Quarkus (today) | `crawl-app` (Hono/Bun) |
|---|---|---|
| HTTP framework | `quarkus-rest` (JAX-RS) | Hono (already scaffolded) |
| Runtime | JVM | Bun (`bun run`), no native image |
| DB access | Hibernate ORM + Panache, autoApply converters | `postgres` (postgres.js) driver + Drizzle ORM schema mirroring the tables in 5.2 exactly (including code-string enum columns as plain `varchar`, converted in application code, not at the DB layer) — **no migrations ever run from this repo** |
| Object storage | `quarkiverse-minio` | `minio` npm package (official MinIO JS SDK) |
| HTML/JSON-LD scraping | JSoup regex + Jackson JSON tree | `fetch` + the same regexes (5.3) + native `JSON.parse` |
| REST client to NetTruyen | MicroProfile REST Client | Plain `fetch()` wrapper (`src/lib/nettruyen-client.ts`) |
| SSE | Mutiny `Multi`/`MultiEmitter` | Hono's `streamSSE` (`hono/streaming`) backed by `Map<string, Set<SSEStreamingApi>>` |
| Scheduled cleanup | `quarkus-scheduler` cron, always running | On-demand, see 5.8 |
| Config | `@ConfigMapping` + `application.properties` | `.env` + a small `zod`-validated loader, fail-fast on missing/invalid required keys |
| Health/observability | `quarkus-smallrye-health`, OTel | Trivial `GET /health` route only; no OTel for v1 |
| Native image / container | GraalVM native binary | Not required; optional `Dockerfile` (`oven/bun` base image) as a nice-to-have, not the primary run mode |
| Concurrency model for a full crawl | `@Blocking` — holds the HTTP thread for the whole crawl | Same: `await` the full pipeline before responding. Matches the documented current limitation; making it async is an explicit non-goal for v1 |

### 5.11 Deployment/portability & auth — decisions

The owner was unavailable to confirm the two open items from the initial
draft; resolved here using this plan's own stated recommendations so task
breakdown isn't blocked. Both are cheap to revisit later.

- **Deployment target: same machine/LAN as Postgres + MinIO** (option a).
  This matches the actual homelab layout observed while grounding this plan
  (`warada_v2` and its Postgres/MinIO run together; the prior Rust attempt's
  `dev/compose.yaml` also assumes local Postgres). `config.ts` targets plain
  `postgres://`/MinIO endpoint strings for v1; no SSH-tunnel or TLS-specific
  config is added now. If remote/tunneled access is needed later, it's an
  additive change to `config.ts` (connection string + optional TLS flags),
  not a redesign.
- **Auth: add a minimal shared-secret header check** on all three crawl
  routes (`X-Crawl-Api-Key`, compared against a required `CRAWL_API_KEY` env
  var; missing/mismatched key → `401 Unauthorized`). Rationale: this app is
  explicitly designed to be portable/run-from-anywhere, and the source
  Quarkus endpoints having no auth was called out as a gap worth closing
  (Section 5.7 already fixes two other silently-missing checks in the same
  spirit). This is a few lines of Hono middleware, not a framework, so it
  doesn't conflict with the "as light as possible" design goal.

### 5.10 Proposed module layout

```
src/
├── index.ts                 # Hono app bootstrap, route registration, startup cleanup pass
├── routes/
│   └── crawl.ts              # POST comic, POST retry, GET progress (SSE)
├── lib/
│   ├── config.ts              # env loading + zod validation, fail-fast
│   ├── db.ts                   # postgres.js client + drizzle instance (no migrations)
│   ├── schema.ts                # drizzle schema mirroring 5.2 exactly
│   ├── minio.ts                 # MinIO client wrapper: ensure-bucket, put/copy/remove/list
│   ├── nettruyen-client.ts      # fetch wrapper for comic page + chapter list + chapter page
│   └── broadcaster.ts           # SSE in-memory pub/sub + active-crawl guard (5.7)
├── services/
│   ├── extractor.ts             # HTML/JSON-LD/regex parsing (5.3) → typed extracted structures
│   ├── tags.ts                   # resolve-or-create tags, increment comic_count (5.6 step 2)
│   └── crawl.ts                  # orchestrates extractor → MinIO rename → DB persist → broadcaster (5.6)
└── scripts/
    └── cleanup.ts                # standalone temp-object cleanup, runnable via `bun run cleanup`
```

## 6. Impact

- New files under `crawl-app/src/` (routes, lib, services, scripts) — all
  greenfield within this repo.
- `package.json` — new dependencies: `drizzle-orm`, `postgres`, `minio`,
  `zod`, plus `drizzle-kit` as a dev dependency (schema typing only, never
  run as a migration tool against the shared DB).
- `.env` / `.env.example` — new file(s) for `NETTRUYEN_URL`, `IMAGE_BUCKET`,
  `IMAGE_BUCKET_TEMP_PATH`, `CLEANUP_ENABLED`, `CLEANUP_MIN_AGE_MINUTES`,
  `CRAWL_API_KEY`, Postgres connection string, MinIO endpoint/credentials.
- No changes to `warada_v2` are in scope — `crawl-app` is a read/write peer
  against the same DB and bucket, never a code dependency of the monolith or
  vice versa.
- `docs/docs-rule.md` lifecycle: this plan will be broken into
  `docs/tasks/T00X-...` files before implementation starts.

## 7. Linked references

- Backlog: [B001-260829-crawl-service-hono-migration](../backlogs/B001-260829-crawl-service-hono-migration.md)
- `ss.md` (repo root) — original Quarkus-targeted requirements/design; still
  useful background, but Section 5.7 above documents where it diverges from
  the real shipped code.
- Ground-truth source read directly for this plan (sibling checkout):
  `/Users/hungnq18/Documents/Projects/github/warada_v2/src/main/java/com/github/warada_v2/{crawl,shared}/**`
  and `src/main/resources/{application.properties,db/migration/*.sql}`.
- Related completes: none yet.

---

## Open Questions (resolved)

Both items originally listed here were resolved autonomously in Section 5.11
(deployment target: same-host/LAN; auth: shared-secret header) since the
owner was unavailable to confirm. Revisit either decision if real-world usage
shows it was wrong — neither is a structural commitment.

## Assumptions made (stated, not blocking — override anytime)

- Postgres access uses `postgres` (postgres.js) + Drizzle for type safety,
  never running `drizzle-kit push`/`generate` against the shared DB.
- MinIO official `minio` npm SDK is sufficient (matches the monolith's own
  vendor choice).
- No GraphQL surface, no OpenTelemetry, no native image, no migration
  execution of any kind — all explicitly out of scope.
- Single-instance operation is acceptable (in-memory SSE broadcaster + active-
  crawl guard, no Redis pub/sub) — reasonable for a tool one operator starts
  on demand.
- Section 5.7's three behavior gaps (404/409 handling) will be implemented
  correctly in `crawl-app` rather than reproduced as-is, since the fix is
  cheap and the broadcaster's `markActive`/`isActive`/`markInactive` methods
  already exist for exactly this purpose (they're just unused today).
- `crawlChapterById` (single-chapter re-crawl by ID) exists in the Quarkus
  `CrawlService` interface but is **not exposed by any REST endpoint** —
  treated as out of scope for v1 since nothing calls it externally today.
