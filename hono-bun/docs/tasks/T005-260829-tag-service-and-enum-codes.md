# T005 — Tag resolution service and enum code mapping

**Status:** done
**Created:** 2026-08-29
**Plan:** [P001-260829-crawl-service-hono-migration](../plans/P001-260829-crawl-service-hono-migration.md)

## 1. Plan link

Implements Section 5.10's `src/services/tags.ts`, porting `TagServiceImpl`'s
`resolveOrCreateTags`/`incrementComicCount` logic (Plan Section 5.6 step 2).
Also implements the shared enum-code mapping helpers needed everywhere the
app writes `crawling_status`/`type` columns (Plan Section 5.2.1) so this
critical wire-format detail lives in exactly one place. Depends on T002
(schema/DB client).

## 2. Impact of work

- `crawl-app/src/services/tags.ts` — new file:
  `resolveOrCreateTags(rawTagNames: string[])`: normalize
  (`trim().toLowerCase()`, dedupe), look up existing tags by
  `normalized_name`, create missing ones (preserving first-seen display
  casing), return the full `Tag` row set. `incrementComicCount(tags)`:
  `comic_count += 1` for each tag (no decrement path, matches current
  behavior).
- `crawl-app/src/lib/enums.ts` — new file: `CrawlStatus` (`INIT`→`"0000"`,
  `STARTED`→`"0001"`, `FINISHED`→`"9999"`) and `ImageType`
  (`THUMB_IMAGE`→`"THUMB"`, `CHAPTER_IMAGE`→`"CHAPTER_IMAGE"`) with
  `toCode`/`fromCode` helpers, per Plan Section 5.2.1 exactly.

## 3. Definition of Done

- [ ] Resolving `["Action", "action", " Action "]` produces exactly one tag
      row (dedup by normalized name), with `name` = the first-seen display
      form.
- [ ] Resolving a mix of existing + new tag names creates only the new ones
      and returns all of them together.
- [ ] `incrementComicCount` increments `comic_count` by exactly 1 per call
      per tag, verified against a real/local DB row.
- [ ] `toCode(CrawlStatus.INIT) === "0000"`, `toCode(CrawlStatus.FINISHED)
      === "9999"`, `toCode(ImageType.THUMB_IMAGE) === "THUMB"` — unit-tested
      directly against Plan 5.2.1's table (this is a compatibility-critical
      detail, not an implementation choice).

## 4. Work history

- 2026-08-29: Task created from plan P001 Section 5.2.1/5.6/5.10, not yet started.
- 2026-08-29: Added tag normalization, reuse/create, count increments, and the
  database-compatible crawl/image code constants with local database coverage.
