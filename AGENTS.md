# Repository agent instructions

- Read `docs/architecture.md` before changing service boundaries, data flow,
  frontend federation, or infrastructure.
- Follow `docs/development.md` for project layout, commands, and coding rules.
- Keep documentation aligned with the checked-in Cargo, Moon, npm, Compose,
  and service configuration. Do not document planned behavior as implemented.
- Preserve unrelated worktree changes. Avoid generated output, secrets, and
  local environment files in changes.
- Run the narrowest relevant check; before finishing, run `npm run check` for
  cross-project changes where practical.
