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
let dockerAvailable = false;
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
    `POSTGRES_PASSWORD=${randomBytes(32).toString("hex")}`,
    `CRAWL_API_KEY=${crawlKey}`,
    `AUTH_ADMIN_API_KEY=${adminKey}`,
    `AUTH_PASSWORD_PEPPER=${randomBytes(32).toString("hex")}`,
    `JWT_SECRETS_DIR=${jwtDir}`,
    `JWT_ISSUER=http://127.0.0.1:${port}/`,
    "JWT_AUDIENCE=warada-e2e",
    `PUBLIC_BASE_URL=http://127.0.0.1:${port}`,
    `GATEWAY_PORT=${port}`,
  ].join("\n"),
);

function run(command, args, options = {}) {
  return new Promise((resolveRun, reject) => {
    const child = spawn(command, args, {
      cwd: root,
      stdio: "inherit",
      ...options,
    });
    child.once("error", reject);
    child.once("exit", (code, signal) => {
      if (code === 0) resolveRun();
      else reject(new Error(`${command} exited with ${signal ?? code}`));
    });
  });
}

function compose(...args) {
  return ["compose", "--env-file", envFile, "--project-name", project, "-f", "infra/compose.yaml", ...args];
}

try {
  const docker = spawnSync("docker", ["compose", "version"], {
    cwd: root,
    stdio: "inherit",
  });
  if (docker.error || docker.status !== 0) {
    throw new Error("Docker Compose is required to run E2E tests");
  }
  dockerAvailable = true;

  console.log(`Starting isolated system stack on http://127.0.0.1:${port}`);
  await run("docker", compose("up", "--build", "--detach", "--wait", "--wait-timeout", "300"));
  await run(
    "docker",
    compose(
      "exec",
      "-T",
      "postgres",
      "psql",
      "-U",
      "warada",
      "-d",
      "warada",
      "-v",
      "ON_ERROR_STOP=1",
      "-c",
      readFileSync(join(root, "e2e/fixtures/seed.sql"), "utf8"),
    ),
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
  if (dockerAvailable) {
    console.log("Removing isolated E2E containers and database volume");
    await run("docker", compose("down", "--volumes", "--remove-orphans")).catch((error) => {
      console.error(error.message);
    });
  }
  rmSync(temp, { recursive: true, force: true });
}
