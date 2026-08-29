import { describe, expect, test, mock, afterEach } from "bun:test";
import {
  extractComicDetail,
  findComicSeriesNode,
  fetchChapterList,
  parseChapterImageCandidates,
  downloadImage,
  downloadChapterImage,
} from "./nettruyen";

const originalFetch = globalThis.fetch;
afterEach(() => {
  globalThis.fetch = originalFetch;
});

function mockFetch(impl: (url: string) => Promise<Response>) {
  globalThis.fetch = mock(impl) as any;
}

describe("findComicSeriesNode", () => {
  test("finds the ComicSeries node inside a @graph array", () => {
    const html = `<script type="application/ld+json">${JSON.stringify({
      "@graph": [
        { "@type": "BreadcrumbList" },
        { "@type": "ComicSeries", name: "Test Comic", genre: ["Action", "Drama"] },
      ],
    })}</script>`;
    const node = findComicSeriesNode(html);
    expect(node?.name).toBe("Test Comic");
  });

  test("returns undefined when no ComicSeries node exists", () => {
    const html = `<script type="application/ld+json">{"@type":"WebPage"}</script>`;
    expect(findComicSeriesNode(html)).toBeUndefined();
  });
});

describe("extractComicDetail", () => {
  test("parses slug, comicId and JSON-LD fields from the comic page", async () => {
    const html = `
      <script>gOpts.comicSlug = 'one-piece'; gOpts.comicId = '123';</script>
      <script type="application/ld+json">${JSON.stringify({
        "@graph": [{ "@type": "ComicSeries", name: "One Piece", author: { name: "Oda" }, genre: ["Action"], image: "http://cdn/img.jpg" }],
      })}</script>`;
    mockFetch(async () => new Response(html, { status: 200 }));

    const detail = await extractComicDetail("one-piece-123");
    expect(detail.slug).toBe("one-piece");
    expect(detail.comicId).toBe("123");
    expect(detail.title).toBe("One Piece");
    expect(detail.author).toBe("Oda");
    expect(detail.genres).toEqual(["Action"]);
    expect(detail.thumbnailUrl).toBe("http://cdn/img.jpg");
  });

  test("throws when the comic page returns a non-200", async () => {
    mockFetch(async () => new Response("nope", { status: 404 }));
    await expect(extractComicDetail("missing")).rejects.toThrow(/HTTP 404/);
  });
});

describe("fetchChapterList", () => {
  test("maps API items to chapter stubs with derived origin fields", async () => {
    mockFetch(async () =>
      Response.json({
        data: [{ chapter_id: 55, chapter_slug: "chap-1", chapter_name: "Chapter 1", chapter_num: 1 }],
      }),
    );
    const stubs = await fetchChapterList("one-piece", "123");
    expect(stubs).toHaveLength(1);
    expect(stubs[0]).toMatchObject({
      chapterId: "55",
      chapterName: "Chapter 1",
      position: 1,
      originPathParams: "one-piece/chap-1/55",
    });
  });
});

describe("parseChapterImageCandidates", () => {
  test("extracts sv1/sv2 pairs in document order", () => {
    const html = `
      <img data-sv1='http://cdn/a1.jpg' data-sv2='http://cdn/a2.jpg' />
      <img data-sv1='http://cdn/b1.jpg' />
    `;
    const candidates = parseChapterImageCandidates(html);
    expect(candidates).toEqual([
      { sv1: "http://cdn/a1.jpg", sv2: "http://cdn/a2.jpg" },
      { sv1: "http://cdn/b1.jpg", sv2: undefined },
    ]);
  });
});

describe("downloadImage", () => {
  test("sends the required NetTruyen CDN header contract", async () => {
    let seenHeaders: Headers | undefined;
    mockFetch(async (_url, opts?: any) => {
      seenHeaders = new Headers(opts?.headers);
      return new Response(new Uint8Array([1, 2, 3]), { status: 200, headers: { "content-type": "image/jpeg" } });
    });
    const img = await downloadImage("http://cdn/x.jpg");
    expect(img.contentType).toBe("image/jpeg");
    expect(img.extension).toBe(".jpg");
    expect(seenHeaders?.get("sec-fetch-mode")).toBe("cors");
    expect(seenHeaders?.get("referer")).toBeTruthy();
  });

  test("throws on non-200", async () => {
    mockFetch(async () => new Response("err", { status: 500 }));
    await expect(downloadImage("http://cdn/x.jpg")).rejects.toThrow(/HTTP 500/);
  });
});

describe("downloadChapterImage", () => {
  test("falls back to sv2 when sv1 fails", async () => {
    mockFetch(async (url: string) => {
      if (url.includes("bad")) return new Response("err", { status: 500 });
      return new Response(new Uint8Array([1]), { status: 200, headers: { "content-type": "image/webp" } });
    });
    const img = await downloadChapterImage({ sv1: "http://cdn/bad.jpg", sv2: "http://cdn/good.webp" });
    expect(img.extension).toBe(".webp");
  });

  test("throws when sv1 fails and there is no sv2", async () => {
    mockFetch(async () => new Response("err", { status: 500 }));
    await expect(downloadChapterImage({ sv1: "http://cdn/bad.jpg" })).rejects.toThrow(/HTTP 500/);
  });
});
