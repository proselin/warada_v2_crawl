# UI workspaces

- `apps/display` is the Rsbuild Module Federation host.
- `apps/activity-remote` is an independently built Rsbuild remote.
- `libs/ui-kit` builds an ESM package and an MF remote with Rslib. It exports
  shared shadcn-inspired Button, Badge, and Card primitives plus status helpers;
  Rstest uses the Rslib adapter to test the shared library.

For local federation development, start each project in a separate terminal:

```sh
npm run dev --workspace=@warada/ui-kit
npm run dev --workspace=@warada/activity-remote
npm run dev --workspace=@warada/display-host
```

The host dev configuration expects the UI-kit and activity manifests at ports
3010 and 3002. The GraphQL service should be available at port 3001.
The host imports the shared UI primitives from the UI-kit federation remote.
Primitive styles and semantic color tokens are shipped with the component
exposure; add variants there rather than copying component CSS into an app.

Production remote URLs are configured at host build time with
`MF_ACTIVITY_REMOTE_URL` and `MF_UI_KIT_REMOTE_URL`; the Compose build uses
`PUBLIC_BASE_URL` and serves both remotes under same-origin `/remotes/...`
paths.
