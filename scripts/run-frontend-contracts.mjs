import { spawnSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);
const loaderPath = path.join(repositoryRoot, "scripts/resolve-typescript-imports.mjs");

const nodeContracts = [
  "packages/web-core/src/features/export/ui/exportDataState.test.ts",
  "packages/web-core/src/features/kanban/ui/kanbanStreamState.test.ts",
  "packages/web-core/src/features/workspace-chat/ui/sessionListState.test.ts",
  "packages/web-core/src/pages/kanban/projectDataState.test.ts",
  "packages/web-core/src/pages/workspaces/agentosOverviewState.test.ts",
  "packages/web-core/src/pages/workspaces/changesPanelState.test.ts",
  "packages/web-core/src/pages/workspaces/gitPanelState.test.ts",
  "packages/web-core/src/pages/workspaces/processListState.test.ts",
  "packages/web-core/src/pages/workspaces/workspaceListState.test.ts",
  "packages/web-core/src/shared/components/logStreamState.test.ts",
  "packages/ui/src/components/activeStateSemantics.test.ts",
  "packages/ui/src/components/agentosBrandContract.test.ts",
  "packages/ui/src/components/agentosOrbitLogoContract.test.ts",
];

const vitestContracts = [
  "packages/web-core/src/shared/lib/diffDataAdapter.test.ts",
];

function runNodeContract(relativePath) {
  return spawnSync(
    process.execPath,
    [
      "--experimental-strip-types",
      `--experimental-loader=${pathToFileURL(loaderPath).href}`,
      relativePath,
    ],
    { cwd: repositoryRoot, encoding: "utf8" },
  );
}

function runVitestContract(relativePath) {
  const pnpmCommand = process.platform === "win32" ? "pnpm.cmd" : "pnpm";
  const command = process.platform === "win32" ? process.env.ComSpec ?? "cmd.exe" : pnpmCommand;
  const args =
    process.platform === "win32"
      ? [
          "/d",
          "/s",
          "/c",
          `${pnpmCommand} --filter @vibe/web-core exec vitest run ${relativePath}`,
        ]
      : ["--filter", "@vibe/web-core", "exec", "vitest", "run", relativePath];

  return spawnSync(
    command,
    args,
    { cwd: repositoryRoot, encoding: "utf8" },
  );
}

let failures = 0;
let passed = 0;

for (const relativePath of nodeContracts) {
  const result = runNodeContract(relativePath);
  if (result.status === 0) {
    passed += 1;
    console.log(`${relativePath}: ok`);
    continue;
  }

  failures += 1;
  console.error(`${relativePath}: failed (exit ${result.status ?? "signal"})`);
  console.error(result.stderr || result.stdout);
}

console.log(`${passed} frontend contracts passed`);

const runAll = process.argv.includes("--all");
for (const relativePath of vitestContracts) {
  if (!runAll) {
    console.log(`${relativePath}: BLOCKED (Vitest lane requires --all)`);
    continue;
  }

  const result = runVitestContract(relativePath);
  if (result.status === 0) {
    console.log(`${relativePath}: ok`);
    continue;
  }

  failures += 1;
  const detail =
    result.error?.message ||
    result.stderr ||
    result.stdout ||
    `exit ${result.status ?? "signal"}`;
  console.error(`${relativePath}: BLOCKED (Vitest unavailable or failed)`);
  console.error(detail);
}

if (failures > 0) {
  process.exitCode = runAll ? 2 : 1;
}
