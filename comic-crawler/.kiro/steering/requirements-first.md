---
inclusion: always
---

# RULE: Requirements First

This project (Warada — a comic reading & management platform: track, add, manage, crawl)
is in the **requirements phase**. No approved requirements draft exists yet.

Until a requirements draft exists at `docs/requirements.md` AND the user has explicitly
marked it approved, the following rule governs every session.

## The Rule

1. **Do not produce solution design.** No architecture diagrams, no schema design, no
   API surface, no crate/module layout, no technology choices, no code — unless the user
   explicitly asks for that specific thing in the current message.

2. **Drive toward the requirements draft.** When the user describes the system, capture
   it as requirements, not as a solution. Convert wishes ("I want to track / add / crawl")
   into:
   - **Actors** (who uses it)
   - **User stories / use cases** (as a <actor>, I want <capability>, so that <value>)
   - **Functional requirements** (what the system must do)
   - **Non-functional requirements** (performance, scale, auth, storage, reliability)
   - **Constraints & assumptions**
   - **Out of scope** (explicitly)
   - **Open questions** (what still needs a decision)

3. **Ask before assuming.** When a requirement is ambiguous, ask a focused question
   instead of inventing an answer. Prefer 1–4 crisp questions over a long guess.

4. **One source of truth.** The requirements draft lives at `docs/requirements.md`.
   Keep it current. Every requirements conversation updates that file.

5. **Gate to the next phase.** Design and implementation are blocked until the user
   says the requirements are approved (e.g. "requirements approved" / "lock the
   requirements"). When they do, record it at the top of `docs/requirements.md`
   (status + date) and only then proceed to design.

## What the user already told us (raw, unprioritized — to be refined into requirements)

- A comic reading website the user can manage.
- Capabilities wanted: **track**, **add**, **crawl from other websites into my DB**.
- Deployment preference: **one server**, but **each module kept separate like a library**
  (modular monolith).
- An existing crawler (Rust + Axum + Postgres, NetTruyen source) already exists in this repo
  and is a likely starting input — but it is INPUT to requirements, not the requirements.

## Explicitly deferred (do NOT design these yet)

- Workspace/crate layout, module boundaries.
- Database schema changes.
- HTTP API endpoints.
- Storage backend choice (local disk vs MinIO/S3).
- Auth / user model.

These were sketched prematurely in an earlier session. They are parked until requirements
are approved.
