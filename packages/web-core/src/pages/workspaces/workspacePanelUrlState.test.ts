import assert from 'node:assert/strict';

import {
  getRightMainPanelModeForUrlPanel,
  getWorkspaceUrlPanelFromLayout,
  resolveWorkspacePanelUrlOverride,
  workspacePanelSearchSchema,
} from './workspacePanelUrlState';

assert.deepEqual(workspacePanelSearchSchema.parse({ panel: 'logs' }), {
  panel: 'logs',
});
assert.equal(
  workspacePanelSearchSchema.parse({ panel: 'unknown' }).panel,
  undefined
);
assert.equal(workspacePanelSearchSchema.parse({}).panel, undefined);

assert.equal(getRightMainPanelModeForUrlPanel('changes'), 'changes');
assert.equal(getRightMainPanelModeForUrlPanel('logs'), 'logs');
assert.equal(getRightMainPanelModeForUrlPanel('preview'), 'preview');
assert.equal(getRightMainPanelModeForUrlPanel('chat'), null);
assert.equal(getRightMainPanelModeForUrlPanel('git'), null);

assert.deepEqual(resolveWorkspacePanelUrlOverride(undefined), null);
assert.deepEqual(resolveWorkspacePanelUrlOverride('logs'), {
  mobileTab: 'logs',
  rightMainPanelMode: 'logs',
});
assert.deepEqual(resolveWorkspacePanelUrlOverride('git'), {
  mobileTab: 'git',
  rightMainPanelMode: null,
});

assert.equal(
  getWorkspaceUrlPanelFromLayout({
    isMobile: true,
    mobileTab: 'workspaces',
    rightMainPanelMode: 'preview',
    currentUrlPanel: 'preview',
  }),
  'workspaces'
);
assert.equal(
  getWorkspaceUrlPanelFromLayout({
    isMobile: false,
    mobileTab: 'logs',
    rightMainPanelMode: 'changes',
    currentUrlPanel: 'chat',
  }),
  'changes'
);
assert.equal(
  getWorkspaceUrlPanelFromLayout({
    isMobile: false,
    mobileTab: 'git',
    rightMainPanelMode: null,
    currentUrlPanel: undefined,
  }),
  'chat'
);
assert.equal(
  getWorkspaceUrlPanelFromLayout({
    isMobile: false,
    mobileTab: 'chat',
    rightMainPanelMode: null,
    currentUrlPanel: 'git',
  }),
  'git'
);

console.log('workspacePanelUrlState contract: ok');
