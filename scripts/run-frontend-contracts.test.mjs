import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import path from "node:path";

const repositoryRoot = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);

const result = spawnSync(
  process.execPath,
  ["scripts/run-frontend-contracts.mjs"],
  {
    cwd: repositoryRoot,
    encoding: "utf8",
  },
);

assert.equal(result.status, 0, result.stderr || result.stdout);
assert.match(result.stdout, /13 frontend contracts passed/);
assert.match(result.stdout, /activeStateSemantics\.test\.ts: ok/);
assert.match(result.stdout, /diffDataAdapter\.test\.ts: BLOCKED/);

console.log("run-frontend-contracts runner contract: ok");
