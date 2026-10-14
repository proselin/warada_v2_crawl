import { pluginModuleFederation } from "@module-federation/rsbuild-plugin";
import { defineConfig } from "@rslib/core";

export default defineConfig({
  lib: [
    {
      format: "esm",
      dts: true,
      output: { distPath: "./dist/esm" },
    },
    {
      format: "mf",
      dts: false,
      output: {
        distPath: "./dist/mf",
        assetPrefix: "/remotes/ui-kit/",
      },
      dev: {
        assetPrefix: "http://localhost:3010/mf/",
      },
      plugins: [
        pluginModuleFederation(
          {
            name: "warada_ui_kit",
            filename: "remoteEntry.js",
            manifest: true,
            exposes: {
              "./status-class": "./src/index.ts",
            },
            dts: {
              generateTypes: { compilerInstance: "tsgo" },
            },
          },
          {},
        ),
      ],
    },
  ],
  output: { target: "web" },
  server: {
    cors: { origin: "http://localhost:5173" },
    port: 3010,
  },
});
