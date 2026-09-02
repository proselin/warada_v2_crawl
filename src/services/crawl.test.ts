import { afterEach, beforeEach, describe, expect, test } from "bun:test";
import { eq } from "drizzle-orm";
import { markActive, markInactive } from "../lib/broadcaster";
import { db, nextId } from "../lib/db";
import { chapters, comicTags, comics, CrawlStatus, images, tags } from "../lib/schema";
import { crawlNettruyenComic, retryFailedChapters } from "./crawl";

const databaseTest = process.env.RUN_DATABASE_TESTS === "true" ? test : test.skip;

beforeEach(async () => {
  if (process.env.RUN_DATABASE_TESTS !== "true") return;
  await db.delete(comicTags);
  await db.delete(images);
  await db.delete(chapters);
  await db.delete(comics);
  await db.delete(tags);
});

afterEach(() => markInactive("active-comic"));

describe("crawl orchestration guards", () => {
  databaseTest("rejects an existing source path before making a network request", async () => {
    const id = await nextId("comic_id_seq");
    await db.insert(comics).values({
      id,
      slug: "existing-comic",
      status: "OnGoing",
      crawlingStatus: CrawlStatus.FINISHED,
      originPathParams: "existing-comic-1",
    });

    expect(await crawlNettruyenComic("existing-comic-1")).toEqual({ status: 409, body: "Conflict" });
  });

  databaseTest("distinguishes unknown and active retry requests", async () => {
    expect(await retryFailedChapters("missing-comic")).toEqual({ status: 404, body: "Not Found" });

    const id = await nextId("comic_id_seq");
    await db.insert(comics).values({
      id,
      slug: "active-comic",
      status: "OnGoing",
      crawlingStatus: CrawlStatus.FINISHED,
    });
    markActive("active-comic");

    expect(await retryFailedChapters("active-comic")).toEqual({ status: 409, body: "Conflict" });
  });

  databaseTest("looks up comics by slug for the SSE route", async () => {
    expect(await import("./crawl").then(({ comicExistsBySlug }) => comicExistsBySlug("missing-comic"))).toBe(false);
    const id = await nextId("comic_id_seq");
    await db.insert(comics).values({
      id,
      slug: "present-comic",
      status: "OnGoing",
      crawlingStatus: CrawlStatus.FINISHED,
    });
    expect(await import("./crawl").then(({ comicExistsBySlug }) => comicExistsBySlug("present-comic"))).toBe(true);
  });
});
