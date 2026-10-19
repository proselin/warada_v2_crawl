import { spawn, spawnSync } from "node:child_process";
import {
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { generateKeyPairSync, randomBytes } from "node:crypto";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const root = resolve(import.meta.dirname, "../..");
const port = process.env.E2E_PORT ?? "18080";
const project = `warada-e2e-${process.pid}`;
const temp = mkdtempSync(join(tmpdir(), "warada-e2e-"));
const jwtDir = join(temp, "jwt");
const adminKey = randomBytes(32).toString("hex");
const crawlKey = randomBytes(32).toString("hex");
let isManagerAvailable = false;
const { privateKey, publicKey } = generateKeyPairSync("rsa", {
  modulusLength: 2048,
  privateKeyEncoding: { type: "pkcs8", format: "pem" },
  publicKeyEncoding: { type: "spki", format: "pem" },
});

mkdirSync(join(jwtDir, "public"), { recursive: true });
writeFileSync(join(jwtDir, "private.pem"), privateKey, { mode: 0o600 });
writeFileSync(join(jwtDir, "public", "current.pem"), publicKey);

const envFile = join(temp, ".env");
writeFileSync(
  envFile,
  [
    `CRAWL_API_KEY=${crawlKey}`,
    `AUTH_ADMIN_API_KEY=${adminKey}`,
    `AUTH_PASSWORD_PEPPER=${randomBytes(32).toString("hex")}`,
    `JWT_SECRETS_DIR=${jwtDir}`,
    `JWT_ISSUER=http://127.0.0.1:${port}/`,
    "JWT_AUDIENCE=warada-e2e",
    `PUBLIC_BASE_URL=http://127.0.0.1:${port}`,
    `GATEWAY_PORT=${port}`,
    `IMAGE_PREFIX=${process.env.IMAGE_PREFIX ?? "warada"}`,
    `IMAGE_TAG=${process.env.IMAGE_TAG ?? "local"}`,
  ].join("\n"),
);

function run(command, args, options = {}) {
  return new Promise((resolveRun, reject) => {
    const { input, ...spawnOptions } = options;
    const child = spawn(command, args, {
      cwd: root,
      stdio: input === undefined ? "inherit" : ["pipe", "inherit", "inherit"],
      ...spawnOptions,
    });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) resolveRun();
      else reject(new Error(`${command} exited with ${signal ?? code}`));
    });
    if (input !== undefined) child.stdin.end(input);
  });
}

function compose(...args) {
  return ["compose", "--env-file", envFile, "--project-name", project, "-f", "infra/compose.yaml", ...args];
}

const containerHandlerCmd = process.env.CONTAINER_HANDLER ?? "docker";

try { 
  const containerManager = spawnSync(containerHandlerCmd, ["compose", "version"], {
    cwd: root,
    stdio: "inherit",
  });
  if (containerManager.error || containerManager.status !== 0) {
    throw new Error(`${containerHandlerCmd} Compose is required to run E2E tests`);
  }
  isManagerAvailable = true;

  console.log(`Starting isolated system stack on http://127.0.0.1:${port}`);
  const useBuiltImages = process.env.E2E_USE_BUILT_IMAGES === "true";
  await run(
    containerHandlerCmd,
    compose(
      "up",
      useBuiltImages ? "--no-build" : "--build",
      "--detach",
      "--wait",
      "--wait-timeout",
      "300",
    ),
  );
  await run(
    containerHandlerCmd,
    compose(
      "exec",
      "-T",
      "crawler",
      "sqlite3",
      "/data/db/warada.sqlite",
    ),
    {
      input: readFileSync(join(root, "e2e/fixtures/seed.sql"), "utf8"),
    },
  );
  console.log("Running browser and service E2E tests");
  await run("npx", ["playwright", "test", "--config=e2e/playwright.config.ts"], {
    env: {
      ...process.env,
      E2E_BASE_URL: `http://127.0.0.1:${port}`,
      E2E_ADMIN_KEY: adminKey,
      E2E_CRAWL_KEY: crawlKey,
    },
  });
} finally {
  if (isManagerAvailable) {
    console.log("Removing isolated E2E containers and database volume");
    await run(containerHandlerCmd, compose("down", "--volumes", "--remove-orphans")).catch((error) => {
      console.error(error.message);
    });
  }
  rmSync(temp, { recursive: true, force: true });
}
