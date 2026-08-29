import { describe, expect, test } from "bun:test";
import { subscribe, broadcast, isActive, markActive, markInactive } from "./broadcaster";
import type { CrawlProgressEvent } from "./broadcaster";

function event(comicSlug: string): CrawlProgressEvent {
  return { type: "processing", message: "x", data: { chapterId: 1, chapterNumber: "Chapter 1", status: null, comicSlug } };
}

describe("broadcaster", () => {
  test("delivers events only to subscribers of the matching slug", () => {
    const received: CrawlProgressEvent[] = [];
    const unsubA = subscribe("slug-a", (e) => received.push(e));
    subscribe("slug-b", (e) => received.push(e));

    broadcast("slug-a", event("slug-a"));
    expect(received).toHaveLength(1);
    expect(received[0]?.data.comicSlug).toBe("slug-a");

    unsubA();
    broadcast("slug-a", event("slug-a"));
    expect(received).toHaveLength(1); // unsubscribed, no new delivery
  });

  test("broadcast to a slug with no subscribers is a no-op", () => {
    expect(() => broadcast("nobody-listening", event("nobody-listening"))).not.toThrow();
  });

  test("active tracking: markActive/isActive/markInactive", () => {
    expect(isActive("comic-x")).toBe(false);
    markActive("comic-x");
    expect(isActive("comic-x")).toBe(true);
    markInactive("comic-x");
    expect(isActive("comic-x")).toBe(false);
  });
});
