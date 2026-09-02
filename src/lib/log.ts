type TraceFields = Record<string, boolean | number | string | null | undefined>;

export function trace(event: string, fields: TraceFields = {}): void {
  console.log(`[trace] ${event}`, fields);
}

export function traceError(event: string, error: unknown, fields: TraceFields = {}): void {
  console.error(`[trace] ${event}`, {
    ...fields,
    error: error instanceof Error ? error.message : String(error),
  });
}
