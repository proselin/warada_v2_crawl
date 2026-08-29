// Env config loader. No zod: a dozen required strings/numbers don't need a
// validation framework — plain checks are shorter and just as clear.

function optional(name: string, fallback: string): string {
  return process.env[name] ?? fallback;
}

function positiveInt(name: string, fallback: number): number {
  const raw = process.env[name];
  if (raw === undefined) return fallback;
  const n = Number(raw);
  if (!Number.isInteger(n) || n <= 0) {
    throw new Error(`Env var ${name} must be a positive integer, got: ${raw}`);
  }
  return n;
}

function bool(name: string, fallback: boolean): boolean {
  const raw = process.env[name];
  if (raw === undefined) return fallback;
  if (raw === "true") return true;
  if (raw === "false") return false;
  throw new Error(`Env var ${name} must be "true" or "false", got: ${raw}`);
}

export const config = {
  nettruyenUrl: optional("NETTRUYEN_URL", "https://nettruyenar.com"),

  imageBucket: optional("IMAGE_BUCKET", "warada-images"),
  imageBucketTempPath: optional("IMAGE_BUCKET_TEMP_PATH", "temp"),

  cleanupEnabled: bool("CLEANUP_ENABLED", true),
  cleanupMinAgeMinutes: positiveInt("CLEANUP_MIN_AGE_MINUTES", 5),

  crawlApiKey: process.env.CRAWL_API_KEY, // undefined => auth disabled (dev convenience)

  // Postgres: no DATABASE_URL yet => fall back to an embedded PGlite
  // instance (zero setup, real Postgres semantics). Set DATABASE_URL to a
  // postgres:// connection string once the shared DB is ready; no code
  // changes needed, same drizzle schema works with both drivers.
  databaseUrl: process.env.DATABASE_URL,

  minioEndpoint: optional("MINIO_ENDPOINT", "localhost"),
  minioPort: positiveInt("MINIO_PORT", 9000),
  minioUseSSL: bool("MINIO_USE_SSL", false),
  minioAccessKey: optional("MINIO_ACCESS_KEY", "minioadmin"),
  minioSecretKey: optional("MINIO_SECRET_KEY", "minioadmin"),
};

// Fail fast for the one var that's actually required to boot at all when a
// real DB is intended (DATABASE_URL absence is fine — that's the PGlite path).
if (process.env.DATABASE_URL !== undefined && process.env.DATABASE_URL.trim() === "") {
  throw new Error("DATABASE_URL is set but empty; unset it to use PGlite or provide a real connection string");
}
