// Tag resolve-or-create + comic_count increment, ported from TagServiceImpl.
// Dedup key is normalizedName (trim().toLowerCase()); first-seen casing wins
// for the display `name`. comic_count only ever increments (no decrement
// path in the real service either — preserved, not "fixed").

import { eq, inArray, sql } from "drizzle-orm";
import { db, nextId } from "../lib/db";
import { tags } from "../lib/schema";

export function normalizeTagName(name: string): string {
  return name.trim().toLowerCase();
}

/** Resolves each tag name to a row (creating it if unseen), returns the tag ids. */
export async function resolveOrCreateTags(names: string[]): Promise<number[]> {
  const byNormalized = new Map<string, string>(); // normalized -> first-seen display name
  for (const name of names) {
    const normalized = normalizeTagName(name);
    if (normalized && !byNormalized.has(normalized)) byNormalized.set(normalized, name.trim());
  }
  if (byNormalized.size === 0) return [];

  const normalizedNames = [...byNormalized.keys()];
  const existing = await db.select().from(tags).where(inArray(tags.normalizedName, normalizedNames));
  const existingByNormalized = new Map(existing.map((t) => [t.normalizedName, t]));

  const ids: number[] = [];
  for (const normalized of normalizedNames) {
    const found = existingByNormalized.get(normalized);
    if (found) {
      ids.push(found.id);
      continue;
    }
    const id = await nextId("tag_id_seq");
    await db.insert(tags).values({ id, name: byNormalized.get(normalized)!, normalizedName: normalized, comicCount: 0 });
    ids.push(id);
  }
  return ids;
}

/** Increments comic_count for every given tag id by 1 (called once per crawled comic). */
export async function incrementComicCount(tagIds: number[]): Promise<void> {
  if (tagIds.length === 0) return;
  await db
    .update(tags)
    .set({ comicCount: sql`${tags.comicCount} + 1` })
    .where(inArray(tags.id, tagIds));
}
