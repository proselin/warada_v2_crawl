import { describe, expect, test, mock } from "bun:test";
import { minioClient, putTempObject, renameToPermanent, listTempObjects, ensureBucketExists } from "./minio";
import { config } from "./config";

describe("minio wrapper", () => {
  test("putTempObject writes under the configured temp prefix", async () => {
    const calls: any[] = [];
    minioClient.putObject = mock((bucket, object, stream, size, meta) => {
      calls.push({ bucket, object, meta });
      return Promise.resolve({ etag: "x", versionId: null } as any);
    }) as any;

    const path = await putTempObject("20260101_000000_thumbnail.jpg", Buffer.from("x"), "image/jpeg");

    expect(path).toBe(`${config.imageBucketTempPath}/20260101_000000_thumbnail.jpg`);
    expect(calls[0].bucket).toBe(config.imageBucket);
    expect(calls[0].object).toBe(path);
    expect(calls[0].meta["Content-Type"]).toBe("image/jpeg");
  });

  test("renameToPermanent copies then removes the temp object (never leaves it persisted)", async () => {
    const order: string[] = [];
    minioClient.copyObject = mock(() => {
      order.push("copy");
      return Promise.resolve({} as any);
    }) as any;
    minioClient.removeObject = mock(() => {
      order.push("remove");
      return Promise.resolve();
    }) as any;

    await renameToPermanent("temp/abc.jpg", "my-slug/thumb/abc.jpg");

    expect(order).toEqual(["copy", "remove"]);
  });

  test("listTempObjects only returns items under the temp prefix", async () => {
    minioClient.listObjectsV2 = mock(() => {
      const { EventEmitter } = require("node:events");
      const emitter = new EventEmitter();
      setTimeout(() => {
        emitter.emit("data", { name: "temp/old.jpg", lastModified: new Date("2020-01-01") });
        emitter.emit("data", { name: "temp/new.jpg", lastModified: new Date() });
        emitter.emit("end");
      }, 0);
      return emitter;
    }) as any;

    const items = await listTempObjects();
    expect(items).toHaveLength(2);
    expect(items.every((i) => i.name.startsWith("temp/"))).toBe(true);
  });

  test("ensureBucketExists creates the bucket only if absent", async () => {
    let created = false;
    minioClient.bucketExists = mock(() => Promise.resolve(false)) as any;
    minioClient.makeBucket = mock(() => {
      created = true;
      return Promise.resolve();
    }) as any;

    await ensureBucketExists();
    expect(created).toBe(true);
  });
});
