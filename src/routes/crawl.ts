import { Hono } from "hono";
import { streamSSE } from "hono/streaming";
import { requireCrawlApiKey } from "../lib/auth";
import { subscribe } from "../lib/broadcaster";
import { comicExistsBySlug, crawlNettruyenComic, retryFailedChapters } from "../services/crawl";

const crawl = new Hono();

crawl.use("*", requireCrawlApiKey);

crawl.post("/nettruyen/comic", async (c) => {
  let payload: { "slug-and-id"?: unknown };
  try {
    payload = await c.req.json();
  } catch {
    return c.text("Invalid JSON body", 400);
  }

  if (typeof payload["slug-and-id"] !== "string" || !payload["slug-and-id"].trim()) {
    return c.text('"slug-and-id" must be a non-empty string', 400);
  }

  const result = await crawlNettruyenComic(payload["slug-and-id"]);
  return c.text(result.body, result.status);
});

crawl.post("/nettruyen/comic/:slug/retry", async (c) => {
  const result = await retryFailedChapters(c.req.param("slug"));
  return c.text(result.body, result.status);
});

crawl.get("/progress/:comicSlug", async (c) => {
  const comicSlug = c.req.param("comicSlug");
  if (!(await comicExistsBySlug(comicSlug))) return c.text("Not Found", 404);

  return streamSSE(c, async (stream) => {
    const unsubscribe = subscribe(comicSlug, (event) => {
      void stream.writeSSE({ data: JSON.stringify(event) });
    });
    stream.onAbort(unsubscribe);
    await new Promise<void>(() => {});
  });
});

export default crawl;
