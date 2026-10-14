# End-to-end tests

The E2E suite uses Playwright against the actual Compose stack. It covers the
dashboard and Module Federation remotes, database-backed catalog data flowing
through crawler gRPC and GraphQL, gateway routes, crawler API-key handling, and
OAuth client-credentials token verification.

## Layout

```text
e2e/
  fixtures/                 Disposable SQLite seed data
  helpers/                  GraphQL helper and isolated-stack runner
  page-objects/             Dashboard browser interactions and assertions
  specs/                    End-to-end test scenarios
  utils/                    Shared test environment configuration
  playwright.config.ts      Playwright configuration
  tsconfig.json             Native TypeScript project
```

## Run

Install Chromium once and run from the repository root:

```sh
npx playwright install chromium
npm run test:e2e
```

The runner generates temporary secrets and JWT keys, starts an isolated Compose
project with its own database and image volumes, loads the fixture, runs
Playwright, then removes containers and volumes. Set `E2E_PORT` to change the
default host port (`18080`). Docker Compose and Chromium are required.
