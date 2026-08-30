import type { MiddlewareHandler } from "hono";
import { config } from "./config";

export const requireCrawlApiKey: MiddlewareHandler = async (c, next) => {
  if (!config.crawlApiKey || c.req.header("X-Crawl-Api-Key") !== config.crawlApiKey) {
    return c.text("Unauthorized", 401);
  }
  await next();
};
