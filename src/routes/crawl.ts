import { Hono } from "hono";
import { streamSSE } from "hono/streaming";
import { requireCrawlApiKey } from "../lib/auth";
import { subscribe } from "../lib/broadcaster";
import { trace, traceError } from "../lib/log";
import { comicExistsBySlug, crawlNettruyenComic, retryFailedChapters } from "../services/crawl";

const crawl = new Hono();

crawl.use("*", requireCrawlApiKey);

crawl.post("/nettruyen/comic", async (c) => {
  let payload: { "slug-and-id"?: unknown };
  try {
    payload = await c.req.json();
  } catch (error) {
    traceError("crawl.request.rejected.invalid-json", error, { path: c.req.path });
    return c.text("Invalid JSON body", 400);
  }

  if (typeof payload["slug-and-id"] !== "string" || !payload["slug-and-id"].trim()) {
    trace("crawl.request.rejected.invalid-payload", { path: c.req.path });
    return c.text('"slug-and-id" must be a non-empty string', 400);
  }

  const slugNId = payload["slug-and-id"];
  trace("crawl.request.received", { slugNId });
  const result = await crawlNettruyenComic(slugNId);
  trace("crawl.request.completed", { slugNId, status: result.status });
  return c.text(result.body, result.status);
});

crawl.post("/nettruyen/comic/:slug/retry", async (c) => {
  const slug = c.req.param("slug");
  trace("crawl.retry.request.received", { slug });
  const result = await retryFailedChapters(slug);
  trace("crawl.retry.request.completed", { slug, status: result.status });
  return c.text(result.body, result.status);
});

crawl.get("/progress/:comicSlug", async (c) => {
  const comicSlug = c.req.param("comicSlug");
  if (!(await comicExistsBySlug(comicSlug))) {
    trace("crawl.progress.rejected.not-found", { comicSlug });
    return c.text("Not Found", 404);
  }

  trace("crawl.progress.connected", { comicSlug });
  return streamSSE(c, async (stream) => {
    const unsubscribe = subscribe(comicSlug, (event) => {
      void stream.writeSSE({ data: JSON.stringify(event) });
    });
    stream.onAbort(() => {
      unsubscribe();
      trace("crawl.progress.disconnected", { comicSlug });
    });
    await new Promise<void>(() => {});
  });
});

export default crawl;
