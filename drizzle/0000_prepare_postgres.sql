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
