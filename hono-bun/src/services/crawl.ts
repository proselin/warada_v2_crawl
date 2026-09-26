// Crawl orchestration, ported from CrawlServiceImpl. Implements the 4
// discrepancy fixes from plan 5.7: 404 on unknown slug (retry + progress
// lookup), 409 on an already-active crawl (isActive/markActive/markInactive
// are now actually wired up), and the real CrawlStatus wire codes.

import { config } from "../lib/config";
import { and, eq } from "drizzle-orm";
import { db, nextId } from "../lib/db";
import { elapsedMs, trace, traceError } from "../lib/log";
import { chapters, comics, comicTags, CrawlStatus, images, ImageType } from "../lib/schema";
import { putTempObject, renameToPermanent } from "../lib/minio";
import { broadcast, isActive, markActive, markInactive } from "../lib/broadcaster";
import {
  downloadChapterImage,
  downloadImage,
  extractComicDetail,
  fetchChapterImageCandidates,
  fetchChapterList,
} from "./nettruyen";
import { resolveOrCreateTags, incrementComicCount } from "./tags";

export type CrawlResult = { status: 200 | 409; body: string };
export type RetryResult = { status: 200 | 404 | 409; body: string };

function timestampTag(): string {
  const now = new Date();
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}_${pad(now.getHours())}${pad(now.getMinutes())}${pad(now.getSeconds())}`;
}

/** Downloads one image to temp/, then renames it to its permanent MinIO path. Returns the permanent path + filename. */
async function pullAndStoreImage(
  url: string,
  uniqueName: string,
  permanentDir: string,
): Promise<{ filePath: string; fileName: string }> {
  const img = await downloadImage(url);
  const fileName = `${timestampTag()}_${uniqueName}${img.extension}`;
  const tempPath = await putTempObject(fileName, img.buffer, img.contentType);
  const filePath = `${permanentDir}/${fileName}`;
  await renameToPermanent(tempPath, filePath);
  return { filePath, fileName };
}

export async function comicExistsBySlug(slug: string): Promise<boolean> {
  const rows = await db.select({ id: comics.id }).from(comics).where(eq(comics.slug, slug)).limit(1);
  return rows.length > 0;
}

export async function crawlNettruyenComic(slugNId: string): Promise<CrawlResult> {
  const startedAt = performance.now();
  if (isActive(slugNId)) {
    trace("crawl.rejected.active", { slugNId, durationMs: elapsedMs(startedAt) });
    return { status: 409, body: "Conflict" };
  }

  const dup = await db.select({ id: comics.id }).from(comics).where(eq(comics.originPathParams, slugNId)).limit(1);
  if (dup.length > 0) {
    trace("crawl.rejected.duplicate", { slugNId, comicId: dup[0]?.id, durationMs: elapsedMs(startedAt) });
    return { status: 409, body: "Conflict" };
  }

  markActive(slugNId);
  trace("crawl.started", { slugNId });
  let resolvedSlug: string | undefined;
  try {
    const detail = await extractComicDetail(slugNId);
    resolvedSlug = detail.slug;
    markActive(resolvedSlug);
    const chapterStubs = await fetchChapterList(detail.slug, detail.comicId);
    const tagIds = await resolveOrCreateTags(detail.genres);
    trace("crawl.metadata.fetched", {
      slug: detail.slug,
      chapterCount: chapterStubs.length,
      tagCount: tagIds.length,
      hasThumbnail: Boolean(detail.thumbnailUrl),
    });

    let thumbImageId: number | undefined;
    if (detail.thumbnailUrl) {
      const { filePath, fileName } = await pullAndStoreImage(detail.thumbnailUrl, "thumbnail", `${detail.slug}/thumb`);
      thumbImageId = await nextId("image_id_seq");
      await db.insert(images).values({
        id: thumbImageId,
        fileName,
        filePath,
        originUrl: detail.thumbnailUrl,
        type: ImageType.THUMB_IMAGE,
      });
    }

    const comicId = await nextId("comic_id_seq");
    await db.transaction(async (tx) => {
      await tx.insert(comics).values({
        id: comicId,
        slug: detail.slug,
        title: detail.title,
        author: detail.author,
        description: detail.description,
        status: "OnGoing",
        chapterCount: chapterStubs.length,
        crawlingStatus: CrawlStatus.FINISHED, // comic-level status only reflects metadata crawl, not chapters (plan 5.6)
        originId: detail.comicId,
        originUrl: `${config.nettruyenUrl}/truyen-tranh/${slugNId}`,
        originPathParams: slugNId,
        thumbImageId,
      });
      for (const tagId of tagIds) {
        await tx.insert(comicTags).values({ comicId, tagId });
      }
      for (const stub of chapterStubs) {
        const chapterId = await nextId("chapter_id_seq");
        await tx.insert(chapters).values({
          id: chapterId,
          comicId,
          chapterNum: stub.chapterName,
          position: stub.position,
          crawlingStatus: CrawlStatus.INIT,
          originUrl: stub.originUrl,
          originPathParams: stub.originPathParams,
        });
      }
    });
    await incrementComicCount(tagIds);
    trace("crawl.metadata.persisted", { slug: detail.slug, comicId, chapterCount: chapterStubs.length });

    const insertedChapters = await db.select().from(chapters).where(eq(chapters.comicId, comicId));
    await runChapterCrawlLoop(detail.slug, insertedChapters);

    trace("crawl.completed", { slug: detail.slug, comicId, durationMs: elapsedMs(startedAt) });
    return { status: 200, body: "Success" };
  } catch (error) {
    traceError("crawl.failed", error, { slugNId, resolvedSlug, durationMs: elapsedMs(startedAt) });
    throw error;
  } finally {
    if (resolvedSlug) markInactive(resolvedSlug);
    markInactive(slugNId);
    trace("crawl.inactive", { slugNId, resolvedSlug });
  }
}

export async function retryFailedChapters(slug: string): Promise<RetryResult> {
  const startedAt = performance.now();
  const [comic] = await db.select().from(comics).where(eq(comics.slug, slug)).limit(1);
  if (!comic) {
    trace("crawl.retry.rejected.not-found", { slug, durationMs: elapsedMs(startedAt) });
    return { status: 404, body: "Not Found" };
  }
  if (isActive(slug)) {
    trace("crawl.retry.rejected.active", { slug, comicId: comic.id, durationMs: elapsedMs(startedAt) });
    return { status: 409, body: "Conflict" };
  }

  markActive(slug);
  try {
    const pending = await db
      .select()
      .from(chapters)
      .where(and(eq(chapters.comicId, comic.id), eq(chapters.crawlingStatus, CrawlStatus.INIT)));
    trace("crawl.retry.started", { slug, comicId: comic.id, chapterCount: pending.length });
    await runChapterCrawlLoop(slug, pending);
    trace("crawl.retry.completed", { slug, comicId: comic.id, chapterCount: pending.length, durationMs: elapsedMs(startedAt) });
    return { status: 200, body: "Retry queued" };
  } catch (error) {
    traceError("crawl.retry.failed", error, { slug, comicId: comic.id, durationMs: elapsedMs(startedAt) });
    throw error;
  } finally {
    markInactive(slug);
    trace("crawl.retry.inactive", { slug });
  }
}

/** Sequential per-chapter crawl: scrape -> download images -> persist. One failure never aborts the rest (plan 5.6 #3). */
async function runChapterCrawlLoop(comicSlug: string, chapterRows: (typeof chapters.$inferSelect)[]): Promise<void> {
  const startedAt = performance.now();
  trace("crawl.chapters.started", { comicSlug, chapterCount: chapterRows.length });
  for (const chapter of chapterRows) {
    const chapterStartedAt = performance.now();
    trace("crawl.chapter.started", { comicSlug, chapterId: chapter.id, chapterNumber: chapter.chapterNum });
    broadcast(comicSlug, {
      type: "processing",
      message: `Crawling chapter ${chapter.chapterNum}`,
      data: { chapterId: chapter.id, chapterNumber: chapter.chapterNum ?? "", status: null, comicSlug },
    });
    try {
      if (!chapter.originUrl) throw new Error("Chapter has no originUrl");
      const candidates = await fetchChapterImageCandidates(chapter.originUrl);
      const permanentDir = `${comicSlug}/chapters/${chapter.chapterNum}`;
      const downloaded = await Promise.all(
        candidates.map(async (candidate, position) => {
          const img = await downloadChapterImage(candidate);
          const fileName = `${timestampTag()}_${position}${img.extension}`;
          const tempPath = await putTempObject(fileName, img.buffer, img.contentType);
          const filePath = `${permanentDir}/${fileName}`;
          await renameToPermanent(tempPath, filePath);
          return { position, fileName, filePath, originUrl: candidate.sv1 };
        }),
      );

      await db.transaction(async (tx) => {
        for (const d of downloaded) {
          const imageId = await nextId("image_id_seq");
          await tx.insert(images).values({
            id: imageId,
            chapterId: chapter.id,
            position: d.position,
            fileName: d.fileName,
            filePath: d.filePath,
            originUrl: d.originUrl,
            type: ImageType.CHAPTER_IMAGE,
          });
        }
        await tx.update(chapters).set({ crawlingStatus: CrawlStatus.FINISHED }).where(eq(chapters.id, chapter.id));
      });
      trace("crawl.chapter.completed", {
        comicSlug,
        chapterId: chapter.id,
        imageCount: downloaded.length,
        durationMs: elapsedMs(chapterStartedAt),
      });

      broadcast(comicSlug, {
        type: "completed",
        message: `Chapter ${chapter.chapterNum} completed`,
        data: { chapterId: chapter.id, chapterNumber: chapter.chapterNum ?? "", status: null, comicSlug },
      });
    } catch (err) {
      traceError("crawl.chapter.failed", err, {
        comicSlug,
        chapterId: chapter.id,
        chapterNumber: chapter.chapterNum,
        durationMs: elapsedMs(chapterStartedAt),
      });
      broadcast(comicSlug, {
        type: "failed",
        message: err instanceof Error ? err.message : String(err),
        data: { chapterId: chapter.id, chapterNumber: chapter.chapterNum ?? "", status: null, comicSlug },
      });
    }
  }
  trace("crawl.chapters.completed", { comicSlug, chapterCount: chapterRows.length, durationMs: elapsedMs(startedAt) });
}
