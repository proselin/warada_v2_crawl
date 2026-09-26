// Thin wrapper around the official MinIO SDK: bucket bootstrap + the
// two-phase temp->permanent rename pattern from CrawlServiceImpl, plus the
// list/remove pair FileTempClearer needs. One implementation — tests mock
// the underlying Minio.Client methods directly rather than through a
// hand-rolled fake/interface layer.

import { Client } from "minio";
import { config } from "./config";

export const minioClient = new Client({
  endPoint: config.minioEndpoint,
  port: config.minioPort,
  useSSL: config.minioUseSSL,
  accessKey: config.minioAccessKey,
  secretKey: config.minioSecretKey,
});

export async function ensureBucketExists(bucket = config.imageBucket) {
  const exists = await minioClient.bucketExists(bucket);
  if (!exists) await minioClient.makeBucket(bucket);
}

/** Uploads to `{IMAGE_BUCKET_TEMP_PATH}/{filename}`; returns the object path. */
export async function putTempObject(
  filename: string,
  stream: NodeJS.ReadableStream | Buffer,
  contentType = "application/octet-stream",
  bucket = config.imageBucket,
): Promise<string> {
  const object = `${config.imageBucketTempPath}/${filename}`;
  await minioClient.putObject(bucket, object, stream as any, undefined, { "Content-Type": contentType });
  return object;
}

/** Copy temp -> permanent, then delete the temp object (rename-on-success). */
export async function renameToPermanent(tempPath: string, permanentPath: string, bucket = config.imageBucket) {
  await minioClient.copyObject(bucket, permanentPath, `/${bucket}/${tempPath}`);
  await minioClient.removeObject(bucket, tempPath);
}

export async function listTempObjects(bucket = config.imageBucket): Promise<Array<{ name: string; lastModified: Date }>> {
  const prefix = `${config.imageBucketTempPath}/`;
  const items: Array<{ name: string; lastModified: Date }> = [];
  const stream = minioClient.listObjectsV2(bucket, prefix, true);
  return new Promise((resolve, reject) => {
    stream.on("data", (item) => {
      if (item.name && item.lastModified) items.push({ name: item.name, lastModified: item.lastModified });
    });
    stream.on("end", () => resolve(items));
    stream.on("error", reject);
  });
}

export async function removeObject(objectName: string, bucket = config.imageBucket) {
  await minioClient.removeObject(bucket, objectName);
}
