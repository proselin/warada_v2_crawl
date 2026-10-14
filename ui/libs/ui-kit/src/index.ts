const knownStatuses = [
  "queued",
  "running",
  "finished",
  "completed",
  "failed",
] as const;

export type StatusClass = (typeof knownStatuses)[number] | "";

export function statusClass(status: unknown): StatusClass {
  const normalized = String(status ?? "").toLowerCase();
  return knownStatuses.find((knownStatus) => knownStatus === normalized) ?? "";
}

export * from "./components";
