// NetTruyen scraping + image download, merged into one file per ponytail
// ladder decision (client + extractor was an artificial split — there's
// only ever one implementation of either). Regex-based HTML/JSON parsing,
// verified against `NettruyenExtractorImpl` (plan section 5.3/5.5).

import { config } from "../lib/config";

export type ExtractedComicDetail = {
  slug: string;
  comicId: string;
  title: string;
  alternateName?: string;
  author?: string;
  description?: string;
  genres: string[];
  thumbnailUrl?: string;
};

export type ExtractedChapterStub = {
  chapterId: string;
  chapterName: string; // display label, stored in chapters.chapter_num
  position: number; // numeric order, stored in chapters.position
  originPathParams: string;
  originUrl: string;
};

export type DownloadedImage = {
  buffer: Buffer;
  contentType: string;
  extension: string;
};

/** GET the comic page and extract slug/comicId/JSON-LD metadata. */
export async function extractComicDetail(slugNId: string): Promise<ExtractedComicDetail> {
  const url = `${config.nettruyenUrl}/truyen-tranh/${slugNId}`;
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Failed to fetch comic page ${url}: HTTP ${res.status}`);
  const html = await res.text();

  const slugMatch = html.match(/gOpts\.comicSlug\s*=\s*(['"])(.*?)\1/);
  const comicIdMatch = html.match(/gOpts\.comicId\s*=\s*(['"])?(\d+)\1?/);
  if (!slugMatch || !comicIdMatch) {
    throw new Error(`Could not extract comicSlug/comicId from ${url}`);
  }
  const slug = slugMatch[2]!;
  const comicId = comicIdMatch[2]!;

  const series = findComicSeriesNode(html);
  if (!series) throw new Error(`Could not find ComicSeries JSON-LD node in ${url}`);

  return {
    slug,
    comicId,
    title: series.name ?? slug,
    alternateName: series.alternateName,
    author: series.author?.name,
    description: series.description,
    genres: Array.isArray(series.genre) ? series.genre : series.genre ? [series.genre] : [],
    thumbnailUrl: series.image,
  };
}

type JsonLdNode = {
  "@type"?: string | string[];
  name?: string;
  alternateName?: string;
  url?: string;
  image?: string;
  author?: { name?: string };
  genre?: string | string[];
  description?: string;
};

/** Flattens every `<script type="application/ld+json">` block's `@graph` and finds the ComicSeries node. */
export function findComicSeriesNode(html: string): JsonLdNode | undefined {
  const blocks = [...html.matchAll(/<script[^>]*type=["']application\/ld\+json["'][^>]*>([\s\S]*?)<\/script>/gi)];
  for (const block of blocks) {
    let parsed: any;
    try {
      parsed = JSON.parse(block[1]!.trim());
    } catch {
      continue;
    }
    const nodes: JsonLdNode[] = Array.isArray(parsed["@graph"]) ? parsed["@graph"] : [parsed];
    const match = nodes.find((n) => {
      const type = n["@type"];
      return type === "ComicSeries" || (Array.isArray(type) && type.includes("ComicSeries"));
    });
    if (match) return match;
  }
  return undefined;
}

/** Fetches the chapter list JSON and maps it to chapter stubs. */
export async function fetchChapterList(slug: string, comicId: string): Promise<ExtractedChapterStub[]> {
  const url = `${config.nettruyenUrl}/Comic/Services/ComicService.asmx/ChapterList?slug=${slug}&comicId=${comicId}`;
  const res = await fetch(url);
  if (!res.ok) throw new Error(`Failed to fetch chapter list for ${slug}: HTTP ${res.status}`);
  const body = (await res.json()) as { data?: Array<Record<string, any>> };
  const data = body.data ?? [];

  return data.map((item) => {
    const chapterSlug = String(item.chapter_slug);
    const chapterId = String(item.chapter_id);
    const originPathParams = `${slug}/${chapterSlug}/${chapterId}`;
    return {
      chapterId,
      chapterName: String(item.chapter_name),
      position: Number(item.chapter_num),
      originPathParams,
      originUrl: `${config.nettruyenUrl}/truyen-tranh/${originPathParams}`,
    };
  });
}

/** Fetches a chapter's page and extracts sv1/sv2 CDN URL pairs, in document (page) order. */
export async function fetchChapterImageCandidates(originUrl: string): Promise<Array<{ sv1: string; sv2?: string }>> {
  const res = await fetch(originUrl);
  if (!res.ok) throw new Error(`Failed to fetch chapter page ${originUrl}: HTTP ${res.status}`);
  const html = await res.text();
  return parseChapterImageCandidates(html);
}

export function parseChapterImageCandidates(html: string): Array<{ sv1: string; sv2?: string }> {
  const results: Array<{ sv1: string; sv2?: string }> = [];
  // Match whole tags containing data-sv1 so sv2 (in either attribute order) is found in the same tag.
  const tagRe = /<[^>]*\bdata-sv1=['"][^'"]+['"][^>]*>/g;
  const sv1Re = /data-sv1=['"]([^'"]+)['"]/;
  const sv2Re = /data-sv2=['"]([^'"]+)['"]/;
  let tag: RegExpExecArray | null;
  while ((tag = tagRe.exec(html))) {
    const sv1 = sv1Re.exec(tag[0])?.[1];
    const sv2 = sv2Re.exec(tag[0])?.[1];
    if (sv1) results.push({ sv1, sv2 });
  }
  return results;
}

/** Downloads one image URL with the exact header contract NetTruyen's CDN requires (plan 5.5). */
export async function downloadImage(url: string): Promise<DownloadedImage> {
  const res = await fetch(url, {
    headers: {
      Origin: config.nettruyenUrl,
      Referer: config.nettruyenUrl,
      Accept: "*/*",
      "Content-Type": "application/octet-stream",
      "Access-Control-Allow-Origin": "*",
      "sec-fetch-mode": "cors",
      "sec-fetch-dest": "empty",
      "sec-fetch-site": "cross-site",
    },
  });
  if (!res.ok) throw new Error(`Failed to download image ${url}: HTTP ${res.status}`);
  const contentType = res.headers.get("content-type") ?? "application/octet-stream";
  const buffer = Buffer.from(await res.arrayBuffer());
  const extension = extensionFromUrl(url) ?? extensionFromContentType(contentType);
  return { buffer, contentType, extension };
}

function extensionFromUrl(url: string): string | undefined {
  const match = new URL(url).pathname.match(/\.[a-zA-Z0-9]+$/);
  return match?.[0];
}

function extensionFromContentType(contentType: string): string {
  const subtype = contentType.split("/")[1]?.split(";")[0];
  return subtype ? `.${subtype}` : ".bin";
}

/** Downloads a chapter page's image trying sv1 first, falling back to sv2 (plan 5.3 step 4). */
export async function downloadChapterImage(candidate: { sv1: string; sv2?: string }): Promise<DownloadedImage> {
  try {
    return await downloadImage(candidate.sv1);
  } catch (err) {
    if (!candidate.sv2) throw err;
    return await downloadImage(candidate.sv2);
  }
}
