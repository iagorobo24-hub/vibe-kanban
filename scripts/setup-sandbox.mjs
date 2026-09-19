import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const devAssetsSeed = path.join(repoRoot, "dev_assets_seed");
const devAssets = path.join(repoRoot, "dev_assets");
const sandboxAssets = path.join(repoRoot, "sandbox_assets");

export function initSandbox(clean = false) {
  if (clean && fs.existsSync(sandboxAssets)) {
    fs.rmSync(sandboxAssets, { recursive: true, force: true });
  }

  if (!fs.existsSync(sandboxAssets)) {
    fs.mkdirSync(sandboxAssets, { recursive: true });
  }

  // Clean old db.sqlite if present to avoid migration version mismatch
  const oldSeed = path.join(sandboxAssets, "db.sqlite");
  if (fs.existsSync(oldSeed)) {
    try { fs.rmSync(oldSeed, { force: true }); } catch {}
  }

  // 2. Preserve credentials, profiles, and config from dev_assets if present
  const filesToCopy = ["credentials.json", "profiles.json", "config.json"];
  for (const file of filesToCopy) {
    const src = path.join(devAssets, file);
    const dest = path.join(sandboxAssets, file);
    if (fs.existsSync(src) && (!fs.existsSync(dest) || clean)) {
      try {
        fs.copyFileSync(src, dest);
      } catch (err) {
        // Non-fatal
      }
    }
  }

  console.log("✅ [Sandbox] Sandbox assets ready at:", sandboxAssets);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const isClean = process.argv.includes("--clean") || process.argv.includes("-c");
  initSandbox(isClean);
}
