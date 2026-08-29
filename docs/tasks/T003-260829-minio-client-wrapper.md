# T003 — MinIO client wrapper

**Status:** pending
**Created:** 2026-08-29
**Plan:** [P001-260829-crawl-service-hono-migration](../plans/P001-260829-crawl-service-hono-migration.md)

## 1. Plan link

Implements Section 5.10's `src/lib/minio.ts`, covering the two-phase object
layout and rename-on-success pattern from Plan Section 5.4. Depends on T001
(config) for bucket/endpoint/credentials.

## 2. Impact of work

- `crawl-app/src/lib/minio.ts` — new file wrapping the official `minio` npm
  SDK: `ensureBucketExists()` (create bucket if absent, called at startup),
  `putTempObject(filename, stream, contentType)` → writes to
  `{IMAGE_BUCKET_TEMP_PATH}/{filename}`, `renameToPermanent(tempPath,
  permanentPath)` → CopyObject + RemoveObject (mirrors `renameObject` in
  `CrawlServiceImpl`), `listTempObjects()` / `removeObject(path)` for cleanup
  (used by T007).
- `crawl-app/package.json` — add `minio` dependency.

## 3. Definition of Done

- [ ] `ensureBucketExists()` creates the bucket if missing, no-ops if present.
- [ ] `putTempObject` writes under the configured temp prefix; `renameToPermanent`
      performs copy+delete atomically enough that no code path ever persists a
      DB row pointing at a `temp/` path (matches Plan 5.4's rename-on-success rule).
- [ ] Integration-style check: put an object to temp, rename it, confirm the
      temp object no longer exists and the permanent object does (via a real
      or local MinIO instance).
- [ ] Bucket name, temp prefix come from `config.ts` (T001), never hardcoded.

## 4. Work history

- 2026-08-29: Task created from plan P001 Section 5.4/5.10, not yet started.
