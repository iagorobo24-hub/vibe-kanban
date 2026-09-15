import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const assetPath = path.join(root, 'packages/ui/src/assets/agentos-orbit-mark.svg');
const markPath = path.join(root, 'packages/ui/src/components/AgentOSMark.tsx');

assert.equal(existsSync(assetPath), true, 'the selected orbit direction needs a master SVG');

const assetSource = readFileSync(assetPath, 'utf8');
const markSource = readFileSync(markPath, 'utf8');

assert.match(assetSource, /viewBox="0 0 512 512"/);
assert.equal((assetSource.match(/<path\b/g) ?? []).length, 3);
assert.match(markSource, /agentos-orbit-mark\.svg/);
assert.doesNotMatch(markSource, />\s*A\s*</);

console.log('agentosOrbitLogoContract: ok');
