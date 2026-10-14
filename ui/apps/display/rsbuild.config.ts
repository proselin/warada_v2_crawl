import { pluginModuleFederation } from "@module-federation/rsbuild-plugin";
import { defineConfig } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";

const activityManifest =
  process.env.MF_ACTIVITY_REMOTE_URL ??
  "http://localhost:3002/mf-manifest.json";
const uiKitManifest =
  process.env.MF_UI_KIT_REMOTE_URL ??
  "http://localhost:3010/mf/mf-manifest.json";

export default defineConfig({
  source: {
    entry: {
      index: "./src/main.tsx",
    },
  },
  html: {
    template: "./src/index.html",
  },
  plugins: [
    pluginReact(),
    pluginModuleFederation({
      name: "warada_host",
      remotes: {
        activity: `warada_activity@${activityManifest}`,
        "ui-kit": `warada_ui_kit@${uiKitManifest}`,
      },
      shared: {
        react: { singleton: true },
        "react-dom": { singleton: true },
      },
      dts: {
        generateTypes: { compilerInstance: "tsgo" },
      },
    }),
  ],
  server: {
    port: 5173,
    proxy: {
      "/graphql": "http://127.0.0.1:3001",
    },
  },
});
