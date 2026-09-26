import { client } from "../lib/db";
import { trace, traceError } from "../lib/log";

try {
  trace("database.prepare.started");
  await client.file(new URL("../../drizzle/0000_prepare_postgres.sql", import.meta.url));
  trace("database.prepare.completed");
} catch (error) {
  traceError("database.prepare.failed", error);
  process.exitCode = 1;
} finally {
  await client.end({ timeout: 5 });
}
