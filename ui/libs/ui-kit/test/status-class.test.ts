import { expect, test } from "@rstest/core";
import { statusClass } from "../src/index.js";

test("returns the supported status class", () => {
  expect(statusClass("RUNNING")).toBe("running");
  expect(statusClass("failed")).toBe("failed");
});

test("does not return arbitrary values as CSS classes", () => {
  expect(statusClass("custom status")).toBe("");
  expect(statusClass(undefined)).toBe("");
});
