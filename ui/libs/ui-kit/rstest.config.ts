import { withRslibConfig } from "@rstest/adapter-rslib";
import { defineConfig } from "@rstest/core";

export default defineConfig({
  extends: withRslibConfig(),
  include: ["test/**/*.test.ts"],
  reporters: process.env.CI
    ? ["default", ["junit", { outputPath: "test-reports/ui-test.xml" }]]
    : ["default"],
  testEnvironment: "node",
});
