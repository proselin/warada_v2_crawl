import { afterEach, describe, expect, mock, test } from "bun:test";
import { config } from "../lib/config";
import { minioClient } from "../lib/minio";
import { cleanupTempObjects } from "./cleanup";

const cleanupEnabled = config.cleanupEnabled;
const originalListObjectsV2 = minioClient.listObjectsV2;
const originalRemoveObject = minioClient.removeObject;

afterEach(() => {
  config.cleanupEnabled = cleanupEnabled;
  minioClient.listObjectsV2 = originalListObjectsV2;
  minioClient.removeObject = originalRemoveObject;
});

describe("cleanupTempObjects", () => {
  test("deletes only objects older than the configured cutoff", async () => {
    minioClient.listObjectsV2 = mock(() => {
      const stream = new EventTarget() as EventTarget & { on: (name: string, listener: (value?: unknown) => void) => void };
      stream.on = (name, listener) => {
        if (name === "data") {
          listener({ name: "temp/old.jpg", lastModified: new Date("2025-12-31T23:59:00Z") });
          listener({ name: "temp/new.jpg", lastModified: new Date("2026-01-01T00:04:00Z") });
        }
        if (name === "end") listener();
      };
      queueMicrotask(() => stream.dispatchEvent(new Event("end")));
      return stream as any;
    }) as any;
    const deleted: string[] = [];
    minioClient.removeObject = mock((_bucket, name) => {
      deleted.push(name);
      return Promise.resolve();
    }) as any;

    const result = await cleanupTempObjects(new Date("2026-01-01T00:05:00Z"));

    expect(result).toMatchObject({ scanned: 2, candidates: 1, deleted: 1, disabled: false });
    expect(deleted).toEqual(["temp/old.jpg"]);
  });

  test("does not scan or delete when cleanup is disabled", async () => {
    config.cleanupEnabled = false;
    minioClient.listObjectsV2 = mock(() => {
      throw new Error("should not scan");
    }) as any;

    expect(await cleanupTempObjects()).toEqual({ scanned: 0, candidates: 0, deleted: 0, disabled: true });
  });
});
