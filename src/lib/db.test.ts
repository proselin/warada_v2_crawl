import { describe, expect, test } from "bun:test";
import { db, ensureSchema, nextId } from "./db";
import { comics, CrawlStatus } from "./schema";
import { eq } from "drizzle-orm";

describe("db (pglite embedded)", () => {
  test("bootstraps schema and round-trips a row using sequence-generated ids", async () => {
    await ensureSchema();

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

  test("nextId never repeats", async () => {
    await ensureSchema();
    const a = await nextId("tag_id_seq");
    const b = await nextId("tag_id_seq");
    expect(a).not.toBe(b);
  });
});
