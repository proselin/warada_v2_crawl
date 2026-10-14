const knownStatuses = new Set([
    "queued",
    "running",
    "finished",
    "completed",
    "failed"
]);
function statusClass(status) {
    const normalized = String(status ?? "").toLowerCase();
    return knownStatuses.has(normalized) ? normalized : "";
}
export { statusClass };
