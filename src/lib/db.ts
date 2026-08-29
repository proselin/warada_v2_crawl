// DB client. Real DATABASE_URL => postgres-js against the shared Postgres
// (never migrated from here). No DATABASE_URL => embedded PGlite (real
// Postgres semantics, zero setup) so the app is runnable/testable before a
// shared DB exists — same drizzle schema, same SQL, just a different driver.
//
// Neither the real Postgres tables nor their `comic_id_seq`/`chapter_id_seq`/
// `image_id_seq`/`tag_id_seq` sequences have an `id` column DEFAULT — the
// Java side calls `nextval()` explicitly before each insert (JPA
// @SequenceGenerator, allocationSize=1). We must do the same via `nextId()`
// so IDs stay compatible with the shared sequences once pointed at the real DB.

import { sql } from "drizzle-orm";
import { drizzle as drizzlePostgres } from "drizzle-orm/postgres-js";
import { drizzle as drizzlePglite } from "drizzle-orm/pglite";
import { PGlite } from "@electric-sql/pglite";
import postgres from "postgres";
import { config } from "./config";
import * as schema from "./schema";

// Verbatim port of warada_v2's real migration SQL (initialize-270925.sql +
// add-tags-table-260304.sql), used ONLY to bootstrap the embedded PGlite
// instance for local dev/tests. Never run against a real DATABASE_URL.
const BOOTSTRAP_SQL = `
CREATE SEQUENCE IF NOT EXISTS chapter_id_seq START WITH 1 INCREMENT BY 1;
CREATE SEQUENCE IF NOT EXISTS comic_id_seq START WITH 1 INCREMENT BY 1;
CREATE SEQUENCE IF NOT EXISTS image_id_seq START WITH 1 INCREMENT BY 1;
CREATE SEQUENCE IF NOT EXISTS tag_id_seq START WITH 1 INCREMENT BY 1;

CREATE TABLE IF NOT EXISTS comics (
  id bigint NOT NULL PRIMARY KEY,
  slug varchar(200) NOT NULL UNIQUE,
  title varchar(255),
  author varchar(255),
  description text,
  status varchar(255) NOT NULL,
  chapter_count integer,
  crawling_status varchar(255) NOT NULL,
  origin_id varchar(255) UNIQUE,
  origin_url varchar(500) UNIQUE,
  origin_path_params varchar(500),
  thumb_image_id bigint UNIQUE,
  created_at timestamp(6),
  updated_at timestamp(6)
);

CREATE TABLE IF NOT EXISTS chapters (
  id bigint NOT NULL PRIMARY KEY,
  comic_id bigint,
  chapter_num varchar(255),
  position integer,
  crawling_status varchar(255),
  type varchar(255),
  origin_url varchar(500) UNIQUE,
  origin_path_params varchar(500) UNIQUE,
  created_at timestamp(6),
  updated_at timestamp(6)
);

CREATE TABLE IF NOT EXISTS images (
  id bigint NOT NULL PRIMARY KEY,
  chapter_id bigint,
  position integer,
  file_name varchar(1000),
  file_path varchar(1000),
  origin_url varchar(500) UNIQUE,
  type varchar(255),
  created_at timestamp(6),
  updated_at timestamp(6)
);

CREATE TABLE IF NOT EXISTS tags (
  id bigint NOT NULL PRIMARY KEY,
  name varchar(255) NOT NULL,
  normalized_name varchar(255) NOT NULL UNIQUE,
  comic_count integer NOT NULL DEFAULT 0,
  created_at timestamp(6),
  updated_at timestamp(6)
);

CREATE TABLE IF NOT EXISTS comic_tags (
  comic_id bigint NOT NULL,
  tag_id bigint NOT NULL,
  PRIMARY KEY (comic_id, tag_id)
);

DO $$ BEGIN
  ALTER TABLE chapters ADD CONSTRAINT fk_chapters_comic FOREIGN KEY (comic_id) REFERENCES comics (id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED;
EXCEPTION WHEN duplicate_object THEN NULL; END $$;
DO $$ BEGIN
  ALTER TABLE images ADD CONSTRAINT fk_images_chapter FOREIGN KEY (chapter_id) REFERENCES chapters (id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED;
EXCEPTION WHEN duplicate_object THEN NULL; END $$;
DO $$ BEGIN
  ALTER TABLE comics ADD CONSTRAINT fk_comics_thumb_image FOREIGN KEY (thumb_image_id) REFERENCES images (id) ON DELETE SET NULL DEFERRABLE INITIALLY DEFERRED;
EXCEPTION WHEN duplicate_object THEN NULL; END $$;
DO $$ BEGIN
  ALTER TABLE comic_tags ADD CONSTRAINT fk_comic_tags_comic FOREIGN KEY (comic_id) REFERENCES comics (id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED;
EXCEPTION WHEN duplicate_object THEN NULL; END $$;
DO $$ BEGIN
  ALTER TABLE comic_tags ADD CONSTRAINT fk_comic_tags_tag FOREIGN KEY (tag_id) REFERENCES tags (id) ON DELETE CASCADE DEFERRABLE INITIALLY DEFERRED;
EXCEPTION WHEN duplicate_object THEN NULL; END $$;
`;

function makeDb() {
  if (config.databaseUrl) {
    const client = postgres(config.databaseUrl);
    return { db: drizzlePostgres({ client, schema }), usingPglite: false as const, client };
  }
  const client = new PGlite();
  return { db: drizzlePglite({ client, schema }), usingPglite: true as const, client };
}

export const { db, usingPglite, client } = makeDb();

let bootstrapped = false;
/** Idempotent; no-ops against a real DATABASE_URL (schema is Liquibase-owned there). */
export async function ensureSchema() {
  if (!usingPglite || bootstrapped) return;
  await client.exec(BOOTSTRAP_SQL);
  bootstrapped = true;
}

/** Mirrors JPA's @SequenceGenerator(allocationSize=1): fetch-then-insert. */
export async function nextId(sequenceName: "comic_id_seq" | "chapter_id_seq" | "image_id_seq" | "tag_id_seq"): Promise<number> {
  const rows = await db.execute(sql.raw(`SELECT nextval('${sequenceName}') AS id`));
  const row = (rows as unknown as Array<{ id: string | number }>)[0] ?? (rows as any).rows?.[0];
  return Number(row.id);
}
