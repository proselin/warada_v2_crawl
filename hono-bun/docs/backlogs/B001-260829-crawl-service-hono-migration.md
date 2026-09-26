# B001 — Migrate warada_v2 crawl subsystem into standalone Hono/Bun service

**Status:** promoted
**Promoted to:** [P001-260829-crawl-service-hono-migration](../plans/P001-260829-crawl-service-hono-migration.md)
**Created:** 2026-08-29

## What

`warada_v2` (Quarkus/Java monolith) contains a crawl subsystem (NetTruyen manga
scraping, image download, MinIO storage, Postgres persistence, SSE progress)
that a prior requirements/design/tasks document (`ss.md`, root of this repo)
already specced out as a standalone **Quarkus** `crawl-service`. That spec
assumes an always-on JVM microservice with GraalVM native builds, Liquibase
awareness, OpenTelemetry, and scheduled cron jobs.

The owner of this repo (`crawl-app`) wants to re-target that extraction: build
the crawl subsystem as a **Hono + Bun** application instead, because a crawl is
an occasional, on-demand action — not a service that needs to run 24/7. Bun
startup is near-instant and the whole app is a handful of files, so "start it,
crawl, stop it" is the natural operating model.

## Why it might matter

- Running a full JVM microservice (with its own Postgres/MinIO client stack,
  health checks, OTel, native image build) just to crawl a comic once in a
  while is heavyweight for something used occasionally.
- Bun + Hono gives a portable single-process app: no native image step, no
  JVM warm-up, trivial to `bun run` on a laptop, a NAS, or a small VPS only
  when a crawl is needed.
- Reusing the existing shared Postgres schema and MinIO bucket (owned by
  `warada_v2`) keeps this a genuine "extraction," not a rewrite of the whole
  manga catalog — the monolith keeps serving GraphQL/CRUD from the same data.

## Notes

- `ss.md` is background only — it specs a **second Quarkus** service. The
  linked plan instead reads the *actual* live source in the sibling
  `warada_v2` checkout (real SQL migrations, real entities/converters, real
  `NettruyenExtractorImpl`/`CrawlServiceImpl`/`CrawlProgressBroadcasterImpl`/
  `FileTempClearer`) as the source of truth, and documents several places
  where `ss.md`'s described behavior does not match what's actually shipped
  today (e.g. `CrawlStatus`/`ImageType` DB codes, missing 404/409 handling).
- A prior, abandoned migration attempt exists at sibling repo
  `warada_v2_crawl` (Rust/Axum/Diesel skeleton, no real crawl logic) —
  confirms the on-demand-service direction predates the Hono/Bun choice but
  contributes no reusable design.
- See the linked plan for the full architecture translation table (Quarkus
  concept → Hono/Bun equivalent) and the two open questions (deployment
  target, auth) that need the owner's confirmation before task breakdown.
