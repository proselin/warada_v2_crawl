# Development guide

## Source of truth

- Root `Cargo.toml`: Rust workspace membership.
- `.moon/workspace.yml`: Moon project registration.
- Root and project `moon.yml`: orchestration tasks.
- Root `package.json`: npm workspaces and common commands.
- `infra/compose.yaml` and `infra/Caddyfile`: deployment topology and routes.
- `docs/architecture.md`: intent, service boundaries, and end-to-end flows.

Do not duplicate versions of service routing or deployment configuration in
application code when the configuration layer already owns it.

## Common commands

Run from the repository root:

```sh
npm install
npm run check       # Rust check, frontend/E2E typecheck, production UI build
npm test            # Rust tests and UI-kit tests
npm run build:all   # Rust and sequential frontend production build
npm run test:e2e    # Disposable Docker Compose stack plus Playwright
```

Rust quality checks can be run individually:

```sh
moon run warada:fmt       # Check Rust formatting
moon run warada:lint      # Run Clippy across all Rust targets
moon run warada:coverage  # HTML coverage report; requires cargo-llvm-cov
```

Coverage warns below 70% and fails below 50% for region, function, and line
coverage across the Rust workspace and in each included Rust source file.
Binary `main.rs` files are excluded because process startup and server
lifecycles are not meaningfully exercised by the unit-test suite. The HTML and
JSON reports are written under `target/llvm-cov/`.

To run frontend projects separately during local development, use the commands
in `ui/README.md` and start the Rust display and crawler services as described
in their READMEs. E2E setup and folder responsibilities are in
`e2e/README.md`; infrastructure setup is in `infra/README.md`.

## CI/CD

GitHub Actions uses Moon's `moon ci` to select and execute affected project
tasks on pull requests. Pushes to `develop` and pull requests from `develop` to
`main` run the full task set. Every push to `develop` also builds all images,
runs full E2E against them, and publishes the tested images only after success.
Rust unit tests produce coverage: results below 70% warn, while any workspace
or per-file metric below 50% fails the checks. A pull request from `develop` to
`main` reruns E2E against the exact SHA-tagged images; merging promotes those
same images without a rebuild. Rust build data and npm dependencies are cached
between workflow runs. Build outputs and test reports are uploaded as workflow
artifacts. The pipeline publishes images but does not deploy them to hosts.
Require the `checks` status check and pull requests in branch protection
settings for both branches.

CI artifacts include the Rust test log, UI and E2E JUnit XML, the Playwright
HTML report and failure traces, and the Rust LLVM coverage HTML/JSON reports.
GitHub Actions caches Rust registry/build data and lockfile-keyed `node_modules`
for faster subsequent runs.

## Change rules

1. Read the relevant source, configuration, and tests before choosing an
   implementation. Follow an existing pattern where it fits.
2. Put code in the owning project. Keep cross-service contracts in the shared
   protobuf crate and shared UI code in `ui/libs/`.
3. Keep changes narrow. Prefer standard library/platform features and existing
   dependencies over new dependencies, abstractions, or configuration knobs.
4. Validate external input and preserve explicit errors at trust boundaries.
   Do not silently convert dependency or persistence errors into success.
5. Never commit credentials, private JWT keys, generated secrets, local `.env`
   files, build output, or test artifacts.
6. Update architecture or operational docs when behavior, ownership, protocol,
   routing, deployment, or developer commands change.
7. Run the smallest relevant check, then `npm run check` and/or `npm test` for
   cross-project behavior when practical. The full E2E suite requires Docker
   Compose and installed Playwright Chromium.

## Rust

- Keep runnable services in `crates/apps/`; shared crates go in `crates/libs/`.
- Use the workspace dependency/build conventions and regenerate protobuf code
  through the `crawler-proto` build script.
- Keep database ownership with the service that owns the data; use gRPC for the
  display-to-crawler contract instead of adding direct display database access.
- Add or update tests with behavior changes. Database integration tests should
  use an isolated SQLite database.

## Frontend

- Frontend source and tool configuration are TypeScript; use `tsgo` for
  typechecking.
- The display host consumes independently built federation remotes. Keep
  exposed module names, manifest URLs, static asset paths, and gateway paths
  aligned.
- Build `ui-kit`, `activity-remote`, and `display-host` sequentially using the
  root scripts to avoid concurrent writes to shared build output.
- Keep the UI-kit tests in its Rstest project.

## E2E

- Put browser scenarios in `e2e/specs/`, seeded data in `e2e/fixtures/`,
  reusable requests in `e2e/helpers/`, browser interactions/assertions in
  `e2e/page-objects/`, and shared test configuration in `e2e/utils/`.
- E2E must exercise the real Compose services. Avoid replacing service
  boundaries with mocks in system tests.
- Use the disposable project runner for system tests. Never point its teardown
  at a developer's normal Compose project or persistent data.
- Do not create external crawl jobs in routine E2E: external content sources
  make tests nondeterministic and can cause unintended requests.

## Documentation and agents

- Keep repo-wide agent instructions short in the root `AGENTS.md`; detailed
  technical facts belong in `docs/`.
- Treat `docs/architecture.md` and `docs/development.md` as maintained
  references. Update them in the same change as the behavior they describe.
- `docs/AGENTS.md` contains the rules for editing this documentation set.
