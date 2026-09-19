import { spawn, execSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { DatabaseSync } from "node:sqlite";
import { initSandbox } from "./setup-sandbox.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const isWindows = process.platform === "win32";

// Parse CLI arguments
const args = process.argv.slice(2);
let agent = "FREEBUFF";
let prompt = "Responde unicamente: PROBA FREEBUFF V2 OK, no modifiques ningun archivo";
let keepWorkspace = false;
let port = 3011;
let timeoutSec = 120;

for (let i = 0; i < args.length; i++) {
  const arg = args[i];
  if (arg === "--keep" || arg === "-k") {
    keepWorkspace = true;
  } else if (arg === "--port" && args[i + 1]) {
    port = parseInt(args[++i], 10);
  } else if (arg === "--prompt" && args[i + 1]) {
    prompt = args[++i];
  } else if (arg === "--timeout" && args[i + 1]) {
    timeoutSec = parseInt(args[++i], 10);
  } else if (!arg.startsWith("-")) {
    agent = arg.toUpperCase();
  }
}

const baseUrl = `http://127.0.0.1:${port}`;
const sandboxDbPath = path.join(repoRoot, "sandbox_assets", "db.v2.sqlite");

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function uuidToBuffer(uuidStr) {
  return Buffer.from(uuidStr.replace(/-/g, ""), "hex");
}

function bufferToUuid(buf) {
  const hex = Buffer.from(buf).toString("hex");
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

async function checkHealth() {
  try {
    const res = await fetch(`${baseUrl}/api/health`, { signal: AbortSignal.timeout(1500) });
    return res.ok;
  } catch {
    return false;
  }
}

function findPidOnPort(targetPort) {
  try {
    const cmd = isWindows
      ? `netstat -ano | findstr :${targetPort}`
      : `lsof -ti :${targetPort}`;
    const output = execSync(cmd, { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] });
    const lines = output.trim().split(/\r?\n/);
    for (const line of lines) {
      if (line.includes("LISTENING") || !isWindows) {
        const parts = line.trim().split(/\s+/);
        const pid = parts[parts.length - 1];
        if (pid && !isNaN(parseInt(pid, 10))) {
          return parseInt(pid, 10);
        }
      }
    }
  } catch {}
  return null;
}

function killPid(pid) {
  if (!pid) return;
  try {
    if (isWindows) {
      execSync(`taskkill /F /PID ${pid}`, { stdio: "ignore" });
    } else {
      process.kill(pid, "SIGKILL");
    }
  } catch {}
}

async function ensureBackendRunning() {
  const alreadyRunning = await checkHealth();
  if (alreadyRunning) {
    console.log(`📡 Backend ya disponible en ${baseUrl} (usando instancia existente)`);
    return { spawned: false };
  }

  console.log(`⚙️  Inicializando banco de pruebas en sandbox_assets...`);
  initSandbox(false);

  console.log(`🚀 Iniciando backend de pruebas en puerto ${port}...`);
  const serverExe = path.join(repoRoot, "target", "debug", isWindows ? "server.exe" : "server");
  const useDirectBinary = fs.existsSync(serverExe);
  const command = useDirectBinary ? serverExe : (isWindows ? "cargo.exe" : "cargo");
  const spawnArgs = useDirectBinary ? [] : ["run", "--bin", "server"];

  const backendProcess = spawn(command, spawnArgs, {
    cwd: repoRoot,
    env: {
      ...process.env,
      AGENTOS_DATA_DIR: "sandbox_assets",
      VK_DATA_DIR: "sandbox_assets",
      BACKEND_PORT: String(port),
      PORT: String(port),
      DISABLE_WORKTREE_CLEANUP: "1",
      RUST_LOG: "info",
    },
    stdio: ["ignore", "pipe", "pipe"],
  });

  backendProcess.stderr?.on("data", (chunk) => {
    const text = chunk.toString();
    if (text.includes("Compiling") || text.includes("Finished")) {
      process.stdout.write(`[Cargo] ${text.trim()}\n`);
    }
  });

  const start = Date.now();
  const maxWait = 120000;
  let healthy = false;

  while (Date.now() - start < maxWait) {
    await sleep(1500);
    healthy = await checkHealth();
    if (healthy) break;
    process.stdout.write(".");
  }
  console.log("");

  if (!healthy) {
    console.error("❌ Error: El backend no arrancó a tiempo en", baseUrl);
    const pid = findPidOnPort(port);
    if (pid) killPid(pid);
    process.exit(1);
  }

  console.log(`✅ Backend en línea y saludable en ${baseUrl}`);
  return { spawned: true, child: backendProcess };
}

async function getOrRegisterRepo() {
  const reposRes = await fetch(`${baseUrl}/api/repos`);
  const reposData = await reposRes.json();
  let repos = reposData.data || [];

  if (repos.length === 0) {
    console.log("📂 Registrando repositorio vibe-kanban en el sandbox...");
    const regRes = await fetch(`${baseUrl}/api/repos`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        path: repoRoot,
        display_name: "vibe-kanban",
      }),
    });
    const regData = await regRes.json();
    if (!regRes.ok || !regData.data) {
      throw new Error(`Fallo al registrar repositorio: ${JSON.stringify(regData)}`);
    }
    repos = [regData.data];
  }

  const repo = repos[0];

  // Set fast no-op scripts for test execution to bypass heavy cargo build
  const noopScript = isWindows ? "powershell -Command exit 0" : "true";
  try {
    await fetch(`${baseUrl}/api/repos/${repo.id}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        setup_script: noopScript,
        cleanup_script: noopScript,
      }),
    });
  } catch {}

  let branch = "iago";
  try {
    const branchesRes = await fetch(`${baseUrl}/api/repos/${repo.id}/branches`);
    const branchesData = await branchesRes.json();
    if (branchesData.data && branchesData.data.length > 0) {
      const found = branchesData.data.find((b) => b.name === "iago" || b.name === "main");
      branch = found ? found.name : branchesData.data[0].name;
    }
  } catch {}

  return { repo, branch };
}

function getSessionProcesses(sessionId) {
  if (!fs.existsSync(sandboxDbPath)) return [];
  try {
    const db = new DatabaseSync(sandboxDbPath, { readOnly: true });
    const sessionBuf = uuidToBuffer(sessionId);
    const rows = db
      .prepare(
        "SELECT id, status, run_reason, started_at, completed_at FROM execution_processes WHERE session_id = ? ORDER BY rowid ASC"
      )
      .all(sessionBuf);
    db.close();
    return rows.map((r) => ({
      id: bufferToUuid(r.id),
      status: r.status,
      run_reason: r.run_reason,
      started_at: r.started_at,
      completed_at: r.completed_at,
    }));
  } catch {
    return [];
  }
}

async function runTest() {
  console.log("\n========================================================");
  console.log("🧪 AgentOS Sandbox Test Runner (Opción B)");
  console.log(`🤖 Agente a probar : ${agent}`);
  console.log(`💬 Prompt           : "${prompt}"`);
  console.log(`🔌 Endpoint         : ${baseUrl}`);
  console.log(`🧹 Auto-cleanup     : ${keepWorkspace ? "NO (--keep activado)" : "SÍ (limpieza automática)"}`);
  console.log("========================================================\n");

  const { spawned, child } = await ensureBackendRunning();

  let workspaceId = null;

  const cleanup = async () => {
    if (workspaceId && !keepWorkspace) {
      console.log(`\n🧹 Limpiando workspace de pruebas ${workspaceId}...`);
      try {
        // Stop any running processes first if any
        const procs = getSessionProcesses(sessionId);
        for (const proc of procs) {
          if (proc.status === "running") {
            try {
              await fetch(`${baseUrl}/api/execution-processes/${proc.id}/stop`, { method: "POST" });
            } catch {}
          }
        }
        await sleep(1000);
        await fetch(`${baseUrl}/api/workspaces/${workspaceId}?delete_branches=true`, {
          method: "DELETE",
        });
        console.log("✅ Workspace de pruebas eliminado correctamente.");
      } catch (err) {
        console.warn("⚠️ No se pudo eliminar workspace:", err.message);
      }
    }

    if (spawned) {
      console.log("🛑 Deteniendo backend de pruebas...");
      const pid = findPidOnPort(port);
      if (pid) {
        killPid(pid);
      } else if (child) {
        try { child.kill(); } catch {}
      }
      console.log("✅ Backend de pruebas detenido.");
    }
  };

  process.on("SIGINT", async () => {
    await cleanup();
    process.exit(130);
  });
  process.on("SIGTERM", async () => {
    await cleanup();
    process.exit(143);
  });

  let sessionId = null;

  try {
    const { repo, branch } = await getOrRegisterRepo();
    console.log(`📦 Repositorio: ${repo.display_name || repo.path} (Rama: ${branch})`);

    console.log("🚀 Creando workspace y disparando intento...");
    const startPayload = {
      name: `sandbox-test-${agent.toLowerCase()}-${Date.now().toString().slice(-4)}`,
      repos: [{ repo_id: repo.id, target_branch: branch }],
      executor_config: {
        executor: agent,
        variant: "DEFAULT",
      },
      prompt,
    };

    const startRes = await fetch(`${baseUrl}/api/workspaces/start`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(startPayload),
    });

    const startData = await startRes.json();
    if (!startRes.ok || !startData.data) {
      throw new Error(`Error al iniciar workspace: ${JSON.stringify(startData)}`);
    }

    const { workspace, execution_process } = startData.data;
    workspaceId = workspace.id;
    sessionId = execution_process.session_id;

    console.log(`📌 Workspace ID : ${workspaceId}`);
    console.log(`💬 Sesión ID    : ${sessionId}`);
    console.log("\n⏳ Esperando ejecución del agente...");

    const logPrefix = sessionId.slice(0, 2);
    const readOffsets = {}; // processId -> byte offset
    let codingAgentStarted = false;
    let finalStatus = "unknown";
    const startTime = Date.now();

    while (Date.now() - startTime < timeoutSec * 1000) {
      await sleep(1500);

      const processes = getSessionProcesses(sessionId);
      const codingProc = processes.find((p) => p.run_reason === "codingagent");
      const currentProc = codingProc || processes[processes.length - 1];

      if (codingProc && !codingAgentStarted) {
        codingAgentStarted = true;
        console.log(`\n🤖 Proceso CodingAgent (${agent}) iniciado [ID: ${codingProc.id}]`);
      }

      // Read logs for all known processes in this session
      for (const proc of processes) {
        const procLogPath = path.join(
          repoRoot,
          "sandbox_assets",
          "sessions",
          logPrefix,
          sessionId,
          "processes",
          `${proc.id}.jsonl`
        );

        if (fs.existsSync(procLogPath)) {
          try {
            const offset = readOffsets[proc.id] || 0;
            const content = fs.readFileSync(procLogPath, "utf8");
            if (content.length > offset) {
              const newContent = content.slice(offset);
              readOffsets[proc.id] = content.length;
              const lines = newContent.trim().split("\n");
              for (const line of lines) {
                try {
                  const parsed = JSON.parse(line);
                  if (parsed.Stdout) process.stdout.write(parsed.Stdout);
                  else if (parsed.Stderr) process.stderr.write(parsed.Stderr);
                } catch {}
              }
            }
          } catch {}
        }
      }

      // Check if coding agent has finished
      if (codingProc) {
        if (
          codingProc.status === "completed" ||
          codingProc.status === "failed" ||
          codingProc.status === "killed"
        ) {
          finalStatus = codingProc.status;
          break;
        }
      }
    }

    console.log("\n--------------------------------------------------------");
    console.log(`🏁 Estado final de ${agent}: ${finalStatus.toUpperCase()}`);
    console.log(`⏱️  Duración: ${((Date.now() - startTime) / 1000).toFixed(1)}s`);

    if (finalStatus === "completed") {
      console.log("🎉 ¡PRUEBA SUPERADA EXITOSAMENTE!");
    } else if (finalStatus === "failed") {
      console.error("❌ La ejecución del agente falló.");
    } else if (finalStatus === "unknown") {
      console.error(`⏱️ Timeout de ${timeoutSec}s alcanzado antes de terminar.`);
    }

    await cleanup();
    process.exit(finalStatus === "completed" ? 0 : 1);
  } catch (error) {
    console.error("💥 Error durante el test:", error.message);
    await cleanup();
    process.exit(1);
  }
}

runTest();
