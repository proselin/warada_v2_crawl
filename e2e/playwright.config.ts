import { defineConfig } from "@playwright/test";
import { e2eBaseUrl } from "./utils/env";

export default defineConfig({
  testDir: "./specs",
  fullyParallel: false,
  workers: 1,
  reporter: process.env.CI
    ? [["list"], ["html", { outputFolder: "playwright-report", open: "never" }]]
    : "list",
  use: {
    baseURL: e2eBaseUrl,
    browserName: "chromium",
    headless: true,
    trace: "retain-on-failure",
  },
});
