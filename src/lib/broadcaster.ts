// SSE progress broadcaster, ported from CrawlProgressBroadcasterImpl.
// One subscriber set per comic slug; events are pushed as they happen.
// Also owns the "is a crawl currently running for this slug" guard
// (markActive/isActive/markInactive existed in the Java code but were never
// wired up anywhere — this port actually calls them, fixing plan 5.7 #2).

export type CrawlProgressEvent = {
  type: "processing" | "completed" | "failed" | "skipped";
  message: string;
  data: {
    chapterId: number;
    chapterNumber: string;
    status: "INIT" | "STARTED" | "FINISHED" | null;
    comicSlug: string;
  };
};

type Subscriber = (event: CrawlProgressEvent) => void;

const subscribers = new Map<string, Set<Subscriber>>();
const activeSlugs = new Set<string>();

export function subscribe(comicSlug: string, listener: Subscriber): () => void {
  let set = subscribers.get(comicSlug);
  if (!set) {
    set = new Set();
    subscribers.set(comicSlug, set);
  }
  set.add(listener);
  return () => {
    set!.delete(listener);
    if (set!.size === 0) subscribers.delete(comicSlug);
  };
}

export function broadcast(comicSlug: string, event: CrawlProgressEvent): void {
  const set = subscribers.get(comicSlug);
  if (!set) return;
  for (const listener of set) listener(event);
}

export function isActive(comicSlug: string): boolean {
  return activeSlugs.has(comicSlug);
}

export function markActive(comicSlug: string): void {
  activeSlugs.add(comicSlug);
}

export function markInactive(comicSlug: string): void {
  activeSlugs.delete(comicSlug);
}
