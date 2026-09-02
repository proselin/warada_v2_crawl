// DB client for the shared Postgres schema. This app never runs migrations or
// other DDL; the schema remains owned by the monolith.
//
// The `comic_id_seq`/`chapter_id_seq`/
// `image_id_seq`/`tag_id_seq` sequences do not have an `id` column DEFAULT —
// Java side calls `nextval()` explicitly before each insert (JPA
// @SequenceGenerator, allocationSize=1). We must do the same via `nextId()`
// so IDs stay compatible with the shared sequences once pointed at the real DB.

import { drizzle } from "drizzle-orm/postgres-js";
import postgres from "postgres";
import { config } from "./config";
import { trace } from "./log";
import * as schema from "./schema";

export const client = postgres(config.databaseUrl);
export const db = drizzle({ client, schema });
trace("database.client.initialized");

/** Mirrors JPA's @SequenceGenerator(allocationSize=1): fetch-then-insert. */
export async function nextId(sequenceName: "comic_id_seq" | "chapter_id_seq" | "image_id_seq" | "tag_id_seq"): Promise<number> {
  const [row] = await client<{ id: string | number }[]>`SELECT nextval(${sequenceName}::regclass) AS id`;
  if (!row) throw new Error(`Could not allocate an ID from ${sequenceName}`);
  const id = Number(row.id);
  trace("database.id.allocated", { sequenceName, id });
  return id;
}
