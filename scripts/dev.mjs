import { spawn, spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const setupScript = path.join(repoRoot, "scripts", "setup-dev-environment.js");
const isWindows = process.platform === "win32";
const packageManager = isWindows ? "pnpm.cmd" : "pnpm";

function readPorts() {
  const result = spawnSync(process.execPath, [setupScript, "get"], {
    cwd: repoRoot,
    encoding: "utf8",
  });

  if (result.status !== 0) {
    process.stderr.write(result.stderr || result.stdout || "Unable to allocate dev ports\n");
    process.exit(result.status ?? 1);
  }

  const lines = result.stdout.trim().split(/\r?\n/);
  try {
    return JSON.parse(lines.at(-1));
  } catch {
    process.stderr.write(`Unable to parse dev ports from setup output:\n${result.stdout}`);
    process.exit(1);
  }
}

const ports = readPorts();
const env = {
  ...process.env,
  FRONTEND_PORT: String(ports.frontend),
  BACKEND_PORT: String(ports.backend),
  PREVIEW_PROXY_PORT: String(ports.preview_proxy),
  VK_ALLOWED_ORIGINS: `http://localhost:${ports.frontend}`,
  VITE_VK_SHARED_API_BASE: process.env.VK_SHARED_API_BASE ?? "",
};

const children = [
  spawn(packageManager, ["run", "backend:dev:watch"], {
    cwd: repoRoot,
    env,
    stdio: "inherit",
  }),
  spawn(packageManager, ["run", "local-web:dev"], {
    cwd: repoRoot,
    env,
    stdio: "inherit",
  }),
];

let shuttingDown = false;
function shutdown(code = 0) {
  if (shuttingDown) return;
  shuttingDown = true;
  for (const child of children) {
    if (!child.killed) child.kill();
  }
  process.exit(code);
}

for (const child of children) {
  child.on("exit", (code, signal) => {
    if (shuttingDown) return;
    const exitCode = code ?? (signal ? 1 : 0);
    shutdown(exitCode);
  });
  child.on("error", (error) => {
    process.stderr.write(`${error.message}\n`);
    shutdown(1);
  });
}

process.on("SIGINT", () => shutdown(0));
process.on("SIGTERM", () => shutdown(0));

