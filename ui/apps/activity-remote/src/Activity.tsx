import { statusClass } from "@warada/ui-kit";
import { useEffect, useState } from "react";

const query = `query Activity {
  crawlJobs { id slug jobType status error }
}`;

type CrawlJob = {
  id: string;
  slug: string;
  jobType: string;
  status: string;
  error: string | null;
};

type GraphQLResponse<T> = {
  data?: T;
  errors?: { message: string }[];
};

async function loadJobs(): Promise<CrawlJob[]> {
  const response = await fetch("/graphql", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({ query }),
  });
  if (!response.ok) throw new Error(`GraphQL request failed (${response.status})`);
  const payload = (await response.json()) as GraphQLResponse<{ crawlJobs: CrawlJob[] }>;
  if (payload.errors?.length) throw new Error(payload.errors[0].message);
  if (!payload.data) throw new Error("GraphQL response is missing data");
  return payload.data.crawlJobs;
}

function errorMessage(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}

export default function Activity() {
  const [jobs, setJobs] = useState<CrawlJob[]>([]);
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let cancelled = false;
    const refresh = async () => {
      try {
        const results = await loadJobs();
        if (!cancelled) {
          setJobs(results);
          setError("");
        }
      } catch (cause) {
        if (!cancelled) setError(errorMessage(cause));
      } finally {
        if (!cancelled) setLoading(false);
      }
    };

    refresh();
    const interval = window.setInterval(refresh, 10000);
    return () => {
      cancelled = true;
      window.clearInterval(interval);
    };
  }, []);

  return (
    <>
      <div className="section-heading">
        <div>
          <p className="eyebrow">SYSTEM ACTIVITY</p>
          <h2>Crawl jobs</h2>
        </div>
        <span className="count">{jobs.length} recent</span>
      </div>
      {error && <div className="notice" role="alert">{error}</div>}
      {jobs.length === 0 ? (
        <p className="empty compact">{loading ? "Loading activity..." : "No crawl jobs yet."}</p>
      ) : (
        <div className="job-list">
          {jobs.slice(0, 5).map((job) => (
            <article className="job" key={job.id}>
              <span className={`dot ${statusClass(job.status)}`} />
              <span className="job-title">{job.slug}</span>
              <span className="job-type">{job.jobType}</span>
              <span className={`status ${statusClass(job.status)}`}>{job.status}</span>
              {job.error && <span className="job-error">{job.error}</span>}
            </article>
          ))}
        </div>
      )}
    </>
  );
}
