const knownStatuses: ReadonlySet<string> = new Set([
  "queued",
  "running",
  "finished",
  "completed",
  "failed",
]);

export function statusClass(status: unknown): string {
  const normalized = String(status ?? "").toLowerCase();
  return knownStatuses.has(normalized) ? normalized : "";
}
