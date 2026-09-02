import { describe, expect, test } from "bun:test";
import { db, nextId } from "./db";
import { comics, CrawlStatus } from "./schema";
import { eq } from "drizzle-orm";

const databaseTest = process.env.RUN_DATABASE_TESTS === "true" ? test : test.skip;

describe("db (Postgres)", () => {
  databaseTest("round-trips a row using sequence-generated ids", async () => {
    const id = await nextId("comic_id_seq");
    expect(typeof id).toBe("number");
    expect(id).toBeGreaterThan(0);

    await db.insert(comics).values({
      id,
      slug: `test-slug-${id}`,
      status: "OnGoing",
      crawlingStatus: CrawlStatus.FINISHED,
    });

    const rows = await db.select().from(comics).where(eq(comics.id, id));
    expect(rows).toHaveLength(1);
    expect(rows[0]?.crawlingStatus).toBe("9999");
  });

  databaseTest("nextId never repeats", async () => {
    const a = await nextId("tag_id_seq");
    const b = await nextId("tag_id_seq");
    expect(a).not.toBe(b);
  });
});
