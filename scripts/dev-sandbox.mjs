import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { initSandbox } from "./setup-sandbox.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const isWindows = process.platform === "win32";
const packageManager = isWindows ? "pnpm.cmd" : "pnpm";

// Ensure sandbox assets are initialized
initSandbox();

const frontendPort = 3010;
const backendPort = 3011;
const previewProxyPort = 3012;

const env = {
  ...process.env,
  AGENTOS_DATA_DIR: "sandbox_assets",
  VK_DATA_DIR: "sandbox_assets",
  PORT: String(frontendPort),
  FRONTEND_PORT: String(frontendPort),
  BACKEND_PORT: String(backendPort),
  PREVIEW_PROXY_PORT: String(previewProxyPort),
  VK_ALLOWED_ORIGINS: `http://localhost:${frontendPort}`,
  VITE_VK_SHARED_API_BASE: process.env.VK_SHARED_API_BASE ?? "",
};

console.log("\n========================================================");
console.log("🧪 AgentOS Sandbox (Banco de Pruebas Dedicado)");
console.log(`🌐 Frontend UI : http://localhost:${frontendPort}`);
console.log(`🔌 Backend API : http://localhost:${backendPort}`);
console.log(`📁 Almacenamiento aislado: sandbox_assets/`);
console.log("========================================================\n");

const children = [
  spawn(packageManager, ["run", "backend:dev:watch"], {
    cwd: repoRoot,
    env,
    stdio: "inherit",
    shell: isWindows,
  }),
  spawn(packageManager, ["run", "local-web:dev"], {
    cwd: repoRoot,
    env,
    stdio: "inherit",
    shell: isWindows,
  }),
];

let shuttingDown = false;
function shutdown(code = 0) {
  if (shuttingDown) return;
  shuttingDown = true;
  for (const child of children) {
    try {
      child.kill();
    } catch {}
  }
  process.exit(code);
}

process.on("SIGINT", () => shutdown(0));
process.on("SIGTERM", () => shutdown(0));
