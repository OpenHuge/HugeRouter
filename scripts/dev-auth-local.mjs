import { spawn } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const envPath = path.join(repoRoot, ".env.local");

function parseEnvFile(filePath) {
  if (!existsSync(filePath)) {
    return {};
  }

  const env = {};
  const contents = readFileSync(filePath, "utf8");

  for (const rawLine of contents.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) {
      continue;
    }

    const separator = line.indexOf("=");
    if (separator <= 0) {
      continue;
    }

    const key = line.slice(0, separator).trim();
    let value = line.slice(separator + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }

    env[key] = value;
  }

  return env;
}

const fileEnv = parseEnvFile(envPath);
const childEnv = {
  ...process.env,
  ...fileEnv,
  CONSOLE_WEB_BASE_URL: fileEnv.CONSOLE_WEB_BASE_URL ?? "http://localhost:3000",
  CONTROL_PLANE_API_ADDR: fileEnv.CONTROL_PLANE_API_ADDR ?? "127.0.0.1:8081",
  CONTROL_PLANE_STORE_MODE: fileEnv.CONTROL_PLANE_STORE_MODE ?? "memory",
  VITE_CONTROL_PLANE_BASE_URL:
    fileEnv.VITE_CONTROL_PLANE_BASE_URL ?? "http://127.0.0.1:8081",
};

const controlPlaneBaseUrl = childEnv.VITE_CONTROL_PLANE_BASE_URL.replace(
  /\/$/,
  "",
);
const consoleBaseUrl = childEnv.CONSOLE_WEB_BASE_URL.replace(/\/$/, "");

function startProcess(label, command, args, options = {}) {
  const child = spawn(command, args, {
    cwd: repoRoot,
    env: childEnv,
    stdio: ["ignore", "pipe", "pipe"],
    ...options,
  });

  child.stdout.on("data", (chunk) => {
    process.stdout.write(`[${label}] ${chunk}`);
  });
  child.stderr.on("data", (chunk) => {
    process.stderr.write(`[${label}] ${chunk}`);
  });
  child.on("exit", (code, signal) => {
    if (shuttingDown) {
      return;
    }

    console.error(`[${label}] exited unexpectedly: ${signal ?? code}`);
    shutdown(code === 0 ? 1 : (code ?? 1));
  });

  return child;
}

async function waitForUrl(label, url, timeoutMs = 90000) {
  const startedAt = Date.now();
  let lastError = null;

  while (Date.now() - startedAt < timeoutMs) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        console.log(`[ready] ${label}: ${url}`);
        return;
      }
      lastError = new Error(`HTTP ${response.status}`);
    } catch (error) {
      lastError = error;
    }

    await new Promise((resolve) => setTimeout(resolve, 500));
  }

  throw new Error(
    `${label} did not become ready at ${url}: ${lastError?.message ?? "unknown error"}`,
  );
}

let shuttingDown = false;
const children = [];

function shutdown(exitCode = 0) {
  if (shuttingDown) {
    return;
  }

  shuttingDown = true;
  for (const child of children) {
    if (!child.killed) {
      child.kill("SIGTERM");
    }
  }

  setTimeout(() => process.exit(exitCode), 300);
}

process.on("SIGINT", () => shutdown(0));
process.on("SIGTERM", () => shutdown(0));

children.push(
  startProcess("control-plane-api", "cargo", [
    "run",
    "-p",
    "control-plane-api",
    "--",
    "serve",
  ]),
);
children.push(
  startProcess("console-web", "pnpm", [
    "--dir",
    "apps/console-web",
    "dev",
    "--host",
    "0.0.0.0",
    "--port",
    "3000",
  ]),
);

try {
  await Promise.all([
    waitForUrl(
      "control-plane-api",
      `${controlPlaneBaseUrl}/api/control-plane/auth/providers`,
    ),
    waitForUrl("console-web", `${consoleBaseUrl}/login`),
  ]);
  console.log(`[open] Use ${consoleBaseUrl}/login for local OAuth callbacks.`);
  console.log(
    "[note] Keep this process running until the OAuth login finishes.",
  );
} catch (error) {
  console.error(error.message);
  shutdown(1);
}
