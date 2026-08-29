# Crawl App Copilot Instructions

## Commands

- Install dependencies: `bun install`
- Run the development server with hot reload: `bun run dev`
- The app serves on `http://localhost:3000`.
- There are currently no configured build, lint, or test scripts, so no full-suite or single-test command exists.

## Architecture

- This is a Bun application using Hono. `src/index.ts` creates the `Hono` app, declares routes, and default-exports the app for Bun to serve.
- Keep routes and their handlers within this Hono application unless a feature creates a clear need for a separate module. The compiler is configured for strict TypeScript and Hono JSX (`react-jsx` with `hono/jsx`).

## Documentation Workflow

- Before beginning feature work, check `docs/backlogs/` and `docs/plans/` for related work.
- Active work follows `backlogs/` -> `plans/` -> `tasks/` -> `completes/`; `intros/` contains current feature reference material outside that lifecycle.
- Every task must link to a plan. Update its dated work history as work progresses, and write a complete document only after all plan tasks are done and verified.
- Use `<PREFIX><NNN>-<YYMMDD>-<short-name>.md` for lifecycle docs, with a sequential, zero-padded ID per document type. Link documents by relative filename.
- Preserve project history: do not delete backlog, plan, task, or complete documents; once written, complete documents are immutable. Update a related intro document when a completed change affects its current-state description.

See `docs/docs-rule.md` for the required fields and templates for each documentation type.
