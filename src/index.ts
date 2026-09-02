import { Hono } from "hono";
import { elapsedMs, trace, traceError } from "./lib/log";
import { ensureBucketExists } from "./lib/minio";
import crawl from "./routes/crawl";
import { cleanupTempObjects } from "./scripts/cleanup";

const app = new Hono();

app.use("*", async (c, next) => {
  const startedAt = performance.now();
  await next();
  trace("http.request.completed", {
    method: c.req.method,
    path: c.req.path,
    status: c.res.status,
    durationMs: elapsedMs(startedAt),
  });
});
app.onError((error, c) => {
  traceError("http.request.failed", error, { method: c.req.method, path: c.req.path });
  return c.text("Internal Server Error", 500);
});
app.get("/health", (c) => c.text("OK"));
app.route("/api/v1/crawl", crawl);

export default app;

async function start() {
  try {
    await ensureBucketExists();
    await cleanupTempObjects();
    console.log(`Server is starting on port ${process.env.PORT ?? 3000}`);
    Bun.serve({ fetch: app.fetch, port: Number(process.env.PORT ?? 3000) });
  } catch (error) {
    console.error("Failed to start the server:", error);
  }
}

if (import.meta.main) {
  await start();
}
