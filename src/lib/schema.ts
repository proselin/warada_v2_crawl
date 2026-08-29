// Drizzle schema mirroring warada_v2's real, live Postgres tables
// (verified against src/main/resources/db/migration/*.sql in the sibling
// warada_v2 checkout — see docs/plans/P001-...#5.2). Column names/types must
// stay byte-for-byte compatible; this repo never issues DDL against a real
// shared DB (see db.ts).

import { bigint, integer, pgTable, primaryKey, text, timestamp, varchar } from "drizzle-orm/pg-core";

// CrawlStatus / ImageType are stored as plain strings in Postgres (the Java
// side converts via CrawlStatusConverter/ImageTypeConverter). These are the
// exact wire codes — do not change without updating warada_v2 too.
export const CrawlStatus = {
  INIT: "0000",
  STARTED: "0001",
  FINISHED: "9999",
} as const;

export const ImageType = {
  THUMB_IMAGE: "THUMB",
  CHAPTER_IMAGE: "CHAPTER_IMAGE",
} as const;

export const comics = pgTable("comics", {
  id: bigint("id", { mode: "number" }).primaryKey(),
  slug: varchar("slug", { length: 200 }).notNull().unique(),
  title: varchar("title", { length: 255 }),
  author: varchar("author", { length: 255 }),
  description: text("description"),
  status: varchar("status", { length: 255 }).notNull(),
  chapterCount: integer("chapter_count"),
  crawlingStatus: varchar("crawling_status", { length: 255 }).notNull(),
  originId: varchar("origin_id", { length: 255 }).unique(),
  originUrl: varchar("origin_url", { length: 500 }).unique(),
  originPathParams: varchar("origin_path_params", { length: 500 }),
  thumbImageId: bigint("thumb_image_id", { mode: "number" }).unique(),
  createdAt: timestamp("created_at", { precision: 6 }),
  updatedAt: timestamp("updated_at", { precision: 6 }),
});

export const chapters = pgTable("chapters", {
  id: bigint("id", { mode: "number" }).primaryKey(),
  comicId: bigint("comic_id", { mode: "number" }),
  chapterNum: varchar("chapter_num", { length: 255 }), // holds the chapter *name/label*, not a number
  position: integer("position"), // holds the numeric chapter_num from NetTruyen
  crawlingStatus: varchar("crawling_status", { length: 255 }),
  type: varchar("type", { length: 255 }),
  originUrl: varchar("origin_url", { length: 500 }).unique(),
  originPathParams: varchar("origin_path_params", { length: 500 }).unique(),
  createdAt: timestamp("created_at", { precision: 6 }),
  updatedAt: timestamp("updated_at", { precision: 6 }),
});

export const images = pgTable("images", {
  id: bigint("id", { mode: "number" }).primaryKey(),
  chapterId: bigint("chapter_id", { mode: "number" }),
  position: integer("position"),
  fileName: varchar("file_name", { length: 1000 }),
  filePath: varchar("file_path", { length: 1000 }),
  originUrl: varchar("origin_url", { length: 500 }).unique(),
  type: varchar("type", { length: 255 }),
  createdAt: timestamp("created_at", { precision: 6 }),
  updatedAt: timestamp("updated_at", { precision: 6 }),
});

export const tags = pgTable("tags", {
  id: bigint("id", { mode: "number" }).primaryKey(),
  name: varchar("name", { length: 255 }).notNull(),
  normalizedName: varchar("normalized_name", { length: 255 }).notNull().unique(),
  comicCount: integer("comic_count").notNull().default(0),
  createdAt: timestamp("created_at", { precision: 6 }),
  updatedAt: timestamp("updated_at", { precision: 6 }),
});

export const comicTags = pgTable(
  "comic_tags",
  {
    comicId: bigint("comic_id", { mode: "number" }).notNull(),
    tagId: bigint("tag_id", { mode: "number" }).notNull(),
  },
  (t) => [primaryKey({ columns: [t.comicId, t.tagId] })],
);
