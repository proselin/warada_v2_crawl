import { Hono } from "hono";
import { ensureSchema } from "./lib/db";
import { ensureBucketExists } from "./lib/minio";
import crawl from "./routes/crawl";
import { cleanupTempObjects } from "./scripts/cleanup";

const app = new Hono();

app.get("/health", (c) => c.text("OK"));
app.route("/api/v1/crawl", crawl);

export default app;

async function start() {
  await ensureSchema();
  await ensureBucketExists();
  await cleanupTempObjects();
  Bun.serve({ fetch: app.fetch, port: Number(process.env.PORT ?? 3000) });
}

if (import.meta.main) {
  await start();
}
