# Documentation agent instructions

- Read `architecture.md` before editing service, protocol, persistence,
  gateway, or frontend federation documentation.
- Read `development.md` before documenting commands or contributor rules.
- Verify factual claims against source/configuration. Mark planned behavior as
  planned; do not describe it as implemented.
- Prefer updating an existing page over adding a near-duplicate. Keep
  operational setup in `../infra/README.md` and E2E usage in `../e2e/README.md`.
- When architecture changes, update both the architecture page and the
  relevant operational/project README in the same change.
- Keep links relative and test commands runnable from the repository root.
