# T008 — Standalone cleanup script

**Status:** pending
**Created:** 2026-08-29
**Plan:** [P001-260829-crawl-service-hono-migration](../plans/P001-260829-crawl-service-hono-migration.md)

## 1. Plan link

Implements Section 5.10's `src/scripts/cleanup.ts`, porting
`FileTempClearer.cleanTempObjects`'s scan+delete logic (Plan Section 5.8) as
a standalone, host-schedulable script rather than an in-process cron.
Depends on T001 (config) and T003 (MinIO).

## 2. Impact of work

- `crawl-app/src/scripts/cleanup.ts` — new file: if `CLEANUP_ENABLED=false`,
  log and exit 0 immediately. Otherwise list all objects under
  `{IMAGE_BUCKET_TEMP_PATH}/` recursively, compute
  `cutoff = now - CLEANUP_MIN_AGE_MINUTES`, delete every object whose
  `lastModified < cutoff`, log scanned/candidate/deleted counts (mirrors
  `FileTempClearer`'s log fields), and exit non-zero if MinIO is unreachable
  (does not throw uncaught — caught, logged at error level, non-zero exit).
- `crawl-app/package.json` — add a `"cleanup": "bun run src/scripts/cleanup.ts"`
  script.
- `crawl-app/src/index.ts` — call this script's exported cleanup function
  once at startup (T007 already lists this wiring; this task owns the
  underlying implementation).

## 3. Definition of Done

- [ ] Given a set of mock/test MinIO objects with `lastModified` values
      straddling the cutoff, exactly the older-than-cutoff ones are deleted
      and the rest are left untouched (matches `ss.md` Property 5's intent).
- [ ] `CLEANUP_ENABLED=false` results in zero delete calls and a log entry
      noting cleanup is disabled.
- [ ] MinIO unreachable during the scan results in a logged error and
      non-zero exit code, without deleting a partial set.
- [ ] `bun run cleanup` runs standalone (without starting the HTTP server).

## 4. Work history

- 2026-08-29: Task created from plan P001 Section 5.8/5.10, not yet started.
