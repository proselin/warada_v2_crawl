# Docs Rule

This document defines how project documentation is organized, written, and linked. It applies to **both AI agents and human contributors** working on this project. Any doc created in `backlogs/`, `plans/`, `tasks/`, `completes/`, or `intros/` must follow the conventions below.

---

## 1. Lifecycle

Work generally flows through the system like this:

```
backlogs  →  plans  →  tasks  →  completes
                                    ↑
                        (multiple tasks can close into one complete)

intros  (standalone, human-facing — not part of the flow above)
```

- A **backlog** item is a candidate for future work — not committed, not scheduled, no current owner. It exists so investigation isn't lost.
- When a backlog item (or a fresh idea) is committed to being worked on, it becomes a **plan** — the design/decision doc for that work.
- A **plan** is broken into one or more **tasks** — the actual units of execution, each linked back to its plan.
- When all tasks for a plan are finished and verified, the work is closed out into a **complete** — the historical record.
- **Intros** sit outside this flow: they're onboarding/reference docs describing what a feature *is*, for people (or agents) ramping up on the project. They get updated when a `complete` ships something worth documenting, but they are not task-tracking docs.

---

## 2. Doc Types

### 2.1 `backlogs/`
**Purpose:** Hold candidate work items that have been identified but are **not currently scheduled**. This is a lookup/parking lot for investigation, not an active commitment.

**Contains:**
- What the idea/problem is
- Why it might matter (rough justification, not a full plan)
- Optional: rough notes from investigation so far

**Promotion rule:** When a backlog item is picked up, create a `plan` that links back to it, then update the backlog file's `Status` to `promoted` and add a link to the new plan. The backlog item stays as historical context; don't delete it.

---

### 2.2 `plans/`
**Purpose:** The core control doc for current and upcoming development. Every active piece of work must have a plan before tasks are created.

**Required fields (all 7):**
1. **Reason** — why this work is happening
2. **Precondition** — what must be true/in place before starting
3. **Post-condition** — what must be true once this is done
4. **Definition of Done** — concrete, checkable criteria
5. **Logic** — the approach/design for how this will be achieved
6. **Impact** — what parts of the system, config, or behavior this touches
7. **Linked references** — related `backlogs/` or `completes/` docs

**Promotion rule:** A plan is broken into one or more `tasks/`, each linking back to this plan file.

**Status transitions:** `draft` → `active` once tasks start; `active` → `done` only when the `complete` doc for this plan has been written (not merely when all tasks are individually marked done — the complete doc is what finalizes it).

---

### 2.3 `tasks/`
**Purpose:** Track current, in-progress execution work. A task is the unit an agent (or human) is actually doing right now.

**Required fields (all 4):**
1. **Plan link** — which plan this task implements (required; if there's truly no plan, state why)
2. **Impact of work** — which files, configs, or systems this task changes (concrete list, updated as work proceeds)
3. **Definition of Done** — task-level completion criteria (should map to a subset of the plan's DoD)
4. **Work history** — a running, dated log of what was actually done (like a changelog: date + short entry per significant step)

**Closing rule:** When finished, the task's outcome feeds into a `completes/` doc for its plan.

---

### 2.4 `completes/`
**Purpose:** The historical record of finished, approved work. Once something here is written, it's closed — treat it as an audit trail, not something to keep editing. **A `completes/` doc is immutable once written; never edit it after the fact.** If something was wrong or changed later, document that in a new plan/task/complete instead.

**Contains:**
- Which `plan` this resolves
- Which `tasks` were part of it (list, with links)
- Final summary of what was actually built/changed (may differ from the original plan — note deviations)
- Link back to any `backlogs` item this originated from, if applicable

---

### 2.5 `intros/`
**Purpose:** Human/agent-facing onboarding and reference docs — "what is this feature and how does it work." Not for tracking active work (that's `plans`/`tasks`), not for historical audit (that's `completes`).

**Contains:**
- Feature overview
- Current state of the feature (high-level, not task-level detail)
- Pointers to relevant `completes/` docs for deeper history, if useful

---

## 3. Naming Convention

Format: `<PREFIX><NNN>-<YYMMDD>-<short-name>.md`

| Doc type | Prefix | Example |
|---|---|---|
| backlogs | `B` | `B001-250715-rate-limit-investigation.md` |
| plans | `P` | `P001-250715-auth-flow-redesign.md` |
| tasks | `T` | `T001-250715-fix-login-race-condition.md` |
| completes | `C` | `C001-250715-auth-flow-redesign.md` |
| intros | `I` | `I001-250715-auth-module-overview.md` |

- `NNN` is a sequential ID **within that doc type** (not global), zero-padded to 3 digits — `001`, `002`, ...
- `YYMMDD` is the creation date of the file.
- `short-name` is a lowercase, hyphenated, short description.
- IDs are never reused, even if a doc is later deleted or superseded.

**Linking:** always link by filename (relative markdown link), e.g.:
```markdown
Linked plan: [P001-250715-auth-flow-redesign](../plans/P001-250715-auth-flow-redesign.md)
```

---

## 4. Templates

### 4.1 Backlog template (`backlogs/B00X-YYMMDD-name.md`)

```markdown
# B00X — <Title>

**Status:** open | promoted
**Promoted to:** (fill in once promoted — link to plan)
**Created:** YYYY-MM-DD

## What
<What the idea/problem is>

## Why it might matter
<Rough justification>

## Notes
<Any investigation done so far, optional>
```

### 4.2 Plan template (`plans/P00X-YYMMDD-name.md`)

```markdown
# P00X — <Title>

**Status:** draft | active | done  <!-- done only once the linked complete doc exists -->
**Created:** YYYY-MM-DD
**Linked backlog:** [B0XX-...](../backlogs/B0XX-....md) (optional)

## 1. Reason
<Why this work is happening>

## 2. Precondition
<What must be true before starting>

## 3. Post-condition
<What must be true once done>

## 4. Definition of Done
- [ ] <criterion 1>
- [ ] <criterion 2>

## 5. Logic
<Approach / design for how this will be achieved>

## 6. Impact
<Files, configs, systems affected>

## 7. Linked references
- Backlog: [...]
- Related completes: [...]
```

### 4.3 Task template (`tasks/T00X-YYMMDD-name.md`)

```markdown
# T00X — <Title>

**Status:** in-progress | blocked | done
**Created:** YYYY-MM-DD
**Plan:** [P0XX-...](../plans/P0XX-....md)

## 1. Plan link
<Restate which plan / why, if no plan exists>

## 2. Impact of work
- `path/to/file.ts` — <what changes>
- `config/x.yaml` — <what changes>

## 3. Definition of Done
- [ ] <criterion>

## 4. Work history
- YYYY-MM-DD: <what was done>
- YYYY-MM-DD: <what was done>
```

### 4.4 Complete template (`completes/C00X-YYMMDD-name.md`)

```markdown
# C00X — <Title>

**Resolves plan:** [P0XX-...](../plans/P0XX-....md)
**Closed:** YYYY-MM-DD

## Tasks included
- [T0XX-...](../tasks/T0XX-....md)
- [T0XX-...](../tasks/T0XX-....md)

## Summary
<What was actually built/changed. Note any deviation from the original plan.>

## Originating backlog
[B0XX-...](../backlogs/B0XX-....md) (if applicable)
```

### 4.5 Intro template (`intros/I00X-YYMMDD-name.md`)

```markdown
# I00X — <Feature Name>

**Last updated:** YYYY-MM-DD

## Overview
<What this feature is, in plain terms>

## Current state
<High-level status — not task-level detail>

## Related
- [C0XX-...](../completes/C0XX-....md)
```

---

## 5. Rules Summary (for agents)

1. Before starting new work, check `backlogs/` and `plans/` for related items.
2. Never start a `task` without a linked `plan`.
3. Always update a task's **Work history** as you go, not only at the end.
4. When closing a task, verify its Definition of Done before marking it done.
5. When all tasks for a plan are done, write a `complete` doc and link all involved tasks.
6. Use the exact naming convention (`<PREFIX><NNN>-<YYMMDD>-<name>.md`) for every new file.
7. Never delete `backlogs`, `completes`, or superseded `plans`/`tasks` — they are the project's history. Never edit a `completes/` doc after it is written.
8. When writing a `complete`, check if any `intros/` doc needs updating to reflect the shipped change — update it in the same pass if so.