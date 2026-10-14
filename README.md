# Warada

Rust services and frontend apps are organized as Moon projects:

```text
crates/
  apps/                 Deployable Rust services
  libs/                 Shared Rust crates
ui/
  apps/                 Frontend apps
  libs/                 Shared frontend packages (when needed)
e2e/
  fixtures/             Disposable PostgreSQL seed data
  helpers/              API helpers and full-stack test runner
  page-objects/         Browser page objects
  specs/                Playwright end-to-end scenarios
  utils/                Shared E2E environment/configuration
  playwright.config.ts  Playwright configuration
  tsconfig.json         E2E TypeScript project
infra/                  Compose, gateway, and container build files
```

Project paths and toolchains are registered in `.moon/workspace.yml`;
project-specific tasks live in each project's `moon.yml`.

Run from the repository root:

```sh
npm run check
npm test
npm run test:e2e
npm run build:all
```

E2E structure and commands are documented in `e2e/README.md`. Infrastructure
setup and operations are in `infra/README.md`.
