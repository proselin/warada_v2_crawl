// Env config loader. No zod: a dozen required strings/numbers don't need a
// validation framework — plain checks are shorter and just as clear.

function optional(name: string, fallback: string): string {
  return process.env[name] ?? fallback;
}

function requiredDatabaseUrl(): string {
  const value = process.env.DATABASE_URL?.trim();
  if (!value) throw new Error("DATABASE_URL must be set to a PostgreSQL connection string");

  try {
    const url = new URL(value);
    if (url.protocol !== "postgres:" && url.protocol !== "postgresql:") {
      throw new Error("invalid protocol");
    }
  } catch {
    throw new Error("DATABASE_URL must be a valid postgres:// or postgresql:// connection string");
  }

  return value;
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

  databaseUrl: requiredDatabaseUrl(),

  minioEndpoint: optional("MINIO_ENDPOINT", "localhost"),
  minioPort: positiveInt("MINIO_PORT", 9000),
  minioUseSSL: bool("MINIO_USE_SSL", false),
  minioAccessKey: optional("MINIO_ACCESS_KEY", "minioadmin"),
  minioSecretKey: optional("MINIO_SECRET_KEY", "minioadmin"),
};
