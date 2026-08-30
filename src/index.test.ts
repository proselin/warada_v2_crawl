import { afterEach, describe, expect, test } from "bun:test";
import { config } from "./lib/config";
import app from "./index";

const apiKey = config.crawlApiKey;
afterEach(() => {
  config.crawlApiKey = apiKey;
});

describe("app bootstrap", () => {
  test("keeps the liveness endpoint public", async () => {
    const response = await app.request("/health");
    expect(response.status).toBe(200);
    expect(await response.text()).toBe("OK");
  });

  test("requires an API key before invoking crawl handlers", async () => {
    const response = await app.request("/api/v1/crawl/nettruyen/comic", {
      method: "POST",
      body: JSON.stringify({ "slug-and-id": "one-piece-1" }),
    });
    expect(response.status).toBe(401);
  });

  test("validates malformed crawl requests after authentication", async () => {
    config.crawlApiKey = "test-key";
    const response = await app.request("/api/v1/crawl/nettruyen/comic", {
      method: "POST",
      headers: { "X-Crawl-Api-Key": "test-key" },
      body: "{}",
    });
    expect(response.status).toBe(400);
  });
});
