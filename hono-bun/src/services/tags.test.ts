import { describe, expect, test, beforeEach } from "bun:test";
import { db } from "../lib/db";
import { tags } from "../lib/schema";
import { resolveOrCreateTags, incrementComicCount, normalizeTagName } from "./tags";

const databaseTest = process.env.RUN_DATABASE_TESTS === "true" ? test : test.skip;

beforeEach(async () => {
  if (process.env.RUN_DATABASE_TESTS !== "true") return;
  await db.delete(tags);
});

describe("normalizeTagName", () => {
  test("trims and lowercases", () => {
    expect(normalizeTagName("  Action  ")).toBe("action");
  });
});

describe("resolveOrCreateTags", () => {
  databaseTest("creates new tags with first-seen display casing", async () => {
    const ids = await resolveOrCreateTags(["Action", "Drama"]);
    expect(ids).toHaveLength(2);
    const rows = await db.select().from(tags);
    expect(rows.map((r) => r.name).sort()).toEqual(["Action", "Drama"]);
  });

  databaseTest("dedupes case-insensitively across calls, reusing the existing row", async () => {
    const first = await resolveOrCreateTags(["Action"]);
    const second = await resolveOrCreateTags(["action"]);
    expect(second).toEqual(first);
    const rows = await db.select().from(tags);
    expect(rows).toHaveLength(1);
    expect(rows[0]?.name).toBe("Action"); // first-seen casing preserved
  });

  databaseTest("dedupes duplicate names within a single call", async () => {
    const ids = await resolveOrCreateTags(["Action", "action", "ACTION"]);
    expect(ids).toHaveLength(1);
  });

  databaseTest("ignores blank names", async () => {
    const ids = await resolveOrCreateTags(["  ", ""]);
    expect(ids).toEqual([]);
  });
});

describe("incrementComicCount", () => {
  databaseTest("increments comic_count for each given tag, leaves others untouched", async () => {
    const [actionId, dramaId] = await resolveOrCreateTags(["Action", "Drama"]);
    await incrementComicCount([actionId!]);
    await incrementComicCount([actionId!]);

    const rows = await db.select().from(tags);
    expect(rows.find((r) => r.id === actionId)?.comicCount).toBe(2);
    expect(rows.find((r) => r.id === dramaId)?.comicCount).toBe(0);
  });

  databaseTest("no-ops on an empty list", async () => {
    await expect(incrementComicCount([])).resolves.toBeUndefined();
  });
});
