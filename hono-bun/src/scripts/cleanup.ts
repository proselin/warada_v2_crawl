import { config } from "../lib/config";
import { listTempObjects, removeObject } from "../lib/minio";

export type CleanupResult = {
  scanned: number;
  candidates: number;
  deleted: number;
  disabled: boolean;
};

/** Deletes stale objects from the transient MinIO prefix. */
export async function cleanupTempObjects(now = new Date()): Promise<CleanupResult> {
  if (!config.cleanupEnabled) {
    console.info("Temp-object cleanup is disabled");
    return { scanned: 0, candidates: 0, deleted: 0, disabled: true };
  }

  const cutoff = new Date(now.getTime() - config.cleanupMinAgeMinutes * 60_000);
  const objects = await listTempObjects();
  const candidates = objects.filter((object) => object.lastModified < cutoff);

  for (const object of candidates) {
    await removeObject(object.name);
  }

  const result = {
    scanned: objects.length,
    candidates: candidates.length,
    deleted: candidates.length,
    disabled: false,
  };
  console.info("Temp-object cleanup complete", result);
  return result;
}

if (import.meta.main) {
  try {
    await cleanupTempObjects();
  } catch (error) {
    console.error("Temp-object cleanup failed", error);
    process.exitCode = 1;
  }
}
