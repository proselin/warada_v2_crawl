import { pluginModuleFederation } from "@module-federation/rsbuild-plugin";
import { defineConfig } from "@rsbuild/core";
import { pluginReact } from "@rsbuild/plugin-react";

const development = process.env.NODE_ENV !== "production";

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
      name: "warada_activity",
      filename: "remoteEntry.js",
      manifest: true,
      exposes: {
        "./Activity": "./src/Activity.tsx",
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
  output: {
    assetPrefix: development ? "http://localhost:3002/" : "/remotes/activity/",
  },
  server: {
    cors: {
      origin: "http://localhost:5173",
    },
    port: 3002,
  },
});
