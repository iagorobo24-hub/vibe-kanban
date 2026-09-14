import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const componentsDirectory = dirname(fileURLToPath(import.meta.url));

function readComponent(name: string): string {
  return readFileSync(join(componentsDirectory, name), "utf8");
}

const appBar = readComponent("AppBar.tsx");
const navbar = readComponent("Navbar.tsx");
const turnNavigationPopup = readComponent("TurnNavigationPopup.tsx");
const workspaceSummary = readComponent("WorkspaceSummary.tsx");

assert.doesNotMatch(
  turnNavigationPopup,
  /bg-brand\/10 border-l-2 border-brand/,
  "the active turn must not use the discarded lateral border treatment",
);
assert.match(
  turnNavigationPopup,
  /aria-current=\{isActive \? "step" : undefined\}/,
  "the active turn must expose current step semantics",
);

assert.equal(
  (appBar.match(/aria-current=\{item\.isActive \? "page" : undefined\}/g) ?? [])
    .length,
  2,
  "active AppBar icon and host navigation must expose current page semantics",
);
assert.match(
  appBar,
  /aria-current=\{\s*item\.activeProjectId === project\.id\s*\?\s*"page"\s*:\s*undefined\s*\}/,
  "active project navigation must expose current page semantics",
);
assert.doesNotMatch(
  appBar,
  /aria-current=["']false["']/,
  "inactive AppBar controls must not expose aria-current=false",
);

assert.match(
  navbar,
  /aria-pressed=\{isActive\}/,
  "panel toggle buttons must expose their active boolean",
);
assert.doesNotMatch(
  navbar,
  /isActive\s*=\s*false/,
  "plain action buttons must omit aria-pressed instead of exposing false",
);
assert.match(
  navbar,
  /aria-controls=\{getMobileTabPanelId\(tab\.id\)\}/,
  "mobile tabs must reference their controlled panels",
);

assert.match(
  workspaceSummary,
  /<button[\s\S]*?aria-pressed=\{isActive\}/,
  "the workspace selection button must expose pressed semantics",
);
assert.match(
  workspaceSummary,
  /isActive \? "bg-brand" : "bg-transparent",[\s\S]*?aria-hidden="true"/,
  "the workspace selection indicator must be hidden from assistive technology",
);

console.log("activeStateSemantics contract: ok");
