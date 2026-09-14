import { spawn } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";
const child = spawn(cargo, ["watch", "-w", "crates", "-x", "run --bin server"], {
  cwd: repoRoot,
  env: {
    ...process.env,
    DISABLE_WORKTREE_CLEANUP: "1",
    RUST_LOG: "debug",
  },
  stdio: "inherit",
});

let shuttingDown = false;
function shutdown(code = 0) {
  if (shuttingDown) return;
  shuttingDown = true;
  if (!child.killed) child.kill();
  process.exit(code);
}

child.on("error", (error) => {
  process.stderr.write(`${error.message}\n`);
  shutdown(1);
});
child.on("exit", (code, signal) => {
  if (shuttingDown) return;
  shutdown(code ?? (signal ? 1 : 0));
});

process.on("SIGINT", () => shutdown(0));
process.on("SIGTERM", () => shutdown(0));

