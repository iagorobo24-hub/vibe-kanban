import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';

const root = process.cwd();
const markPath = path.join(root, 'packages/ui/src/components/AgentOSMark.tsx');
const wordmarkPath = path.join(
  root,
  'packages/web-core/src/shared/components/AgentOSWordmark.tsx'
);
const appBarPath = path.join(root, 'packages/ui/src/components/AppBar.tsx');

assert.equal(existsSync(markPath), true, 'the provisional mark must have one shared source');

const markSource = readFileSync(markPath, 'utf8');
const wordmarkSource = readFileSync(wordmarkPath, 'utf8');
const appBarSource = readFileSync(appBarPath, 'utf8');

assert.match(markSource, /export function AgentOSMark/);
assert.match(wordmarkSource, /@vibe\/ui\/components\/AgentOSMark/);
assert.match(wordmarkSource, /<AgentOSMark/);
assert.match(appBarSource, /from ["']\.\/AgentOSMark["']/);
assert.match(appBarSource, /<AgentOSMark/);
assert.doesNotMatch(wordmarkSource, />\s*A\s*</);
assert.doesNotMatch(appBarSource, />\s*A\s*</);

console.log('agentosBrandContract: ok');
