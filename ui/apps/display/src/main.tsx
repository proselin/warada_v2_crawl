import { lazy, StrictMode, Suspense, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { Badge, Button, Card } from "ui-kit/components";
import { statusClass } from "ui-kit/status-class";
import "./style.css";

const Activity = lazy(() => import("activity/Activity"));

type Comic = {
  id: string;
  slug: string;
  title: string;
  author: string | null;
  description: string | null;
  status: string;
  chapterCount: number | null;
  crawlingStatus: string;
};

type Chapter = {
  id: string;
  chapterNumber: number;
  crawlingStatus: string;
};

type GraphQLResponse<T> = {
  data?: T;
  errors?: { message: string }[];
};

const dashboardQuery = `
  query Dashboard {
    comics {
      id slug title author description status chapterCount crawlingStatus
    }
  }
`;

async function queryGraphQL<T>(
  query: string,
  variables: Record<string, unknown> = {},
): Promise<T> {
  const response = await fetch("/graphql", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ query, variables }),
  });
  if (!response.ok) throw new Error(`GraphQL request failed (${response.status})`);
  const payload = (await response.json()) as GraphQLResponse<T>;
  if (payload.errors?.length) throw new Error(payload.errors[0].message);
  if (!payload.data) throw new Error("GraphQL response is missing data");
  return payload.data;
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

function App() {
  const [dashboard, setDashboard] = useState<{ comics: Comic[] }>({ comics: [] });
  const [selectedSlug, setSelectedSlug] = useState<string | null>(null);
  const [chapters, setChapters] = useState<Chapter[]>([]);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    async function refresh() {
      try {
        const data = await queryGraphQL<{ comics: Comic[] }>(dashboardQuery);
        if (!cancelled) {
          setDashboard(data);
          setError("");
          setLoading(false);
        }
      } catch (cause) {
        if (!cancelled) {
          setError(errorMessage(cause));
          setLoading(false);
        }
      }
    }
    refresh();
    const interval = window.setInterval(refresh, 10000);
    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, []);

  useEffect(() => {
    let cancelled = false;
    setChapters([]);
    if (!selectedSlug) return () => { cancelled = true; };
    queryGraphQL<{ chapters: Chapter[] }>(
      "query Chapters($slug: String!) { chapters(comicSlug: $slug) { id chapterNumber crawlingStatus } }",
      { slug: selectedSlug },
    )
      .then((data) => {
        if (!cancelled) setChapters(data.chapters);
      })
      .catch((cause) => {
        if (!cancelled) setError(errorMessage(cause));
      });
    return () => { cancelled = true; };
  }, [selectedSlug]);

  return (
    <main className="page">
      <header className="topbar">
        <a className="brand" href="/">warada<span>.</span></a>
        <span className="live"><i /> Live dashboard</span>
      </header>

      <section className="intro">
        <p className="eyebrow">YOUR READING WORLD</p>
        <h1>Stories, <em>in progress.</em></h1>
        <p className="subhead">Browse the library and keep an eye on every crawl.</p>
      </section>

      {error && <div className="notice" role="alert">{error}</div>}

      <section className="jobs-section">
        <Suspense fallback={<p className="empty compact">Loading crawl activity...</p>}>
          <Activity />
        </Suspense>
      </section>

      <section className="library-section">
        <div className="section-heading">
          <div>
            <p className="eyebrow">EXPLORE</p>
            <h2>The library</h2>
          </div>
          <span className="count">{dashboard.comics.length} titles</span>
        </div>
        {dashboard.comics.length === 0 ? (
          <p className="empty">{loading ? "Loading the library..." : "No comics found."}</p>
        ) : (
          <div className="comic-grid">
            {dashboard.comics.map((comic) => {
              const expanded = selectedSlug === comic.slug;
              return (
                <Card as="article" className="comic-card" key={comic.id}>
                  <div className="card-top">
                    <span className="comic-mark">{comic.title.slice(0, 1).toUpperCase()}</span>
                    <Badge variant={statusClass(comic.crawlingStatus) || "secondary"}>
                      {comic.crawlingStatus}
                    </Badge>
                  </div>
                  <h3>{comic.title}</h3>
                  <p className="author">{comic.author || "Unknown author"}</p>
                  <p className="description">{comic.description || "No description available."}</p>
                  <div className="card-meta">
                    <span>{comic.chapterCount ?? 0} chapters</span>
                    <span>{comic.status}</span>
                  </div>
                  <Button
                    variant="ghost"
                    className="chapters-button"
                    onClick={() => setSelectedSlug(expanded ? null : comic.slug)}
                    aria-expanded={expanded}
                  >
                    {expanded ? "Hide chapters" : "View chapters"}
                    <span aria-hidden="true">{expanded ? "−" : "+"}</span>
                  </Button>
                  {expanded && (
                    <ol className="chapter-list">
                      {chapters.length === 0
                        ? <li>No chapters available.</li>
                        : chapters.map((chapter) => (
                          <li key={chapter.id}>
                            <span>Chapter {chapter.chapterNumber}</span>
                            <span className="chapter-status">{chapter.crawlingStatus}</span>
                          </li>
                        ))}
                    </ol>
                  )}
                </Card>
              );
            })}
          </div>
        )}
      </section>
      <footer>Warada library <span>·</span> Updates every 10 seconds</footer>
    </main>
  );
}

const root = document.getElementById("root");
if (!root) throw new Error("Missing #root element");
createRoot(root).render(<StrictMode><App /></StrictMode>);
