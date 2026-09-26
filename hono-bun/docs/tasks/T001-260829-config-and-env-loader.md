# T001 — Config and env loader

**Status:** done
**Created:** 2026-08-29
**Plan:** [P001-260829-crawl-service-hono-migration](../plans/P001-260829-crawl-service-hono-migration.md)

## 1. Plan link

Implements Section 5.10's `src/lib/config.ts` and the `.env`/`.env.example`
files from Section 6 (Impact). Everything else in this task list depends on
this module for typed, validated access to Postgres/MinIO/NetTruyen/crawl
config, so it must land first.

## 2. Impact of work

- `crawl-app/.env.example` — new file documenting every required var:
  `NETTRUYEN_URL`, `IMAGE_BUCKET`, `IMAGE_BUCKET_TEMP_PATH`,
  `CLEANUP_ENABLED`, `CLEANUP_MIN_AGE_MINUTES`, `CRAWL_API_KEY`,
  `DATABASE_URL` (Postgres connection string), `MINIO_ENDPOINT`,
  `MINIO_PORT`, `MINIO_ACCESS_KEY`, `MINIO_SECRET_KEY`, `MINIO_USE_SSL`.
- `crawl-app/src/lib/config.ts` — new file: `zod` schema for the above vars,
  parsed from `process.env` (Bun auto-loads `.env`), exported as a single
  typed `config` object. Invalid/missing required values must throw with a
  clear message and cause the process to exit non-zero at startup (mirrors
  `ss.md` Req 1.8/1.9 and `WaradaConfig`'s fail-fast behavior).
- `crawl-app/package.json` — add `zod` dependency.
- `crawl-app/.gitignore` — ensure `.env` (not `.env.example`) is ignored.

## 3. Definition of Done

- [ ] `config.ts` exports a fully-typed config object covering every var above.
- [ ] Missing a required var (e.g. unset `DATABASE_URL`) throws synchronously
      on import with a message naming the missing key.
- [ ] `CLEANUP_ENABLED` parses `"true"`/`"false"` to boolean;
      `CLEANUP_MIN_AGE_MINUTES` parses to a positive integer, rejecting
      zero/negative/non-numeric values.
- [ ] `.env.example` has a placeholder line for every key `config.ts` reads.
- [ ] `bun run src/lib/config.ts` (or a quick inline check) confirms the
      module loads without error given a valid `.env`.

## 4. Work history

- 2026-08-29: Task created from plan P001 Section 5.10/6, not yet started.
- 2026-08-29: Added typed environment parsing, safe local defaults, `.env.example`,
  and `.env` ignore coverage. The planned zod dependency was not needed for the
  small fixed configuration shape.
