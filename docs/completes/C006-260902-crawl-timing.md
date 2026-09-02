# C006 — Crawl timing

**Resolves plan:** [P006-260902-crawl-timing](../plans/P006-260902-crawl-timing.md)
**Closed:** 2026-09-02

## Tasks included

- [T013-260902-crawl-timing](../tasks/T013-260902-crawl-timing.md)

## Summary

Added native `durationMs` trace fields for requests, database sequence allocation, complete crawl and retry runs, chapter batches, and individual chapters. This uses the existing console trace path with no logging or metrics dependency.
