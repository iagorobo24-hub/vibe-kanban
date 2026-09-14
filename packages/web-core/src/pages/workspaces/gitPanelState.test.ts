import assert from 'node:assert/strict';
import { deriveGitPanelStatus, findRepoBranchStatus } from './gitPanelState';
import type { RepoBranchStatus } from 'shared/types';

const status = (repoId: string): RepoBranchStatus =>
  ({ repo_id: repoId }) as RepoBranchStatus;

assert.equal(
  deriveGitPanelStatus({
    repoIds: ['repo-1'],
    branchStatus: undefined,
    isLoading: true,
    hasError: false,
  }),
  'loading'
);

assert.equal(
  deriveGitPanelStatus({
    repoIds: ['repo-1'],
    branchStatus: undefined,
    isLoading: false,
    hasError: true,
  }),
  'unavailable'
);

assert.equal(
  deriveGitPanelStatus({
    repoIds: ['repo-1', 'repo-2'],
    branchStatus: [status('repo-1'), status('repo-2')],
    isLoading: false,
    hasError: false,
  }),
  'ready'
);

assert.equal(
  deriveGitPanelStatus({
    repoIds: ['repo-1', 'repo-2'],
    branchStatus: [status('repo-1')],
    isLoading: false,
    hasError: false,
  }),
  'unavailable'
);

assert.equal(
  deriveGitPanelStatus({
    repoIds: [],
    branchStatus: undefined,
    isLoading: true,
    hasError: false,
  }),
  'ready'
);

assert.equal(
  findRepoBranchStatus([status('repo-1')], 'repo-1')?.repo_id,
  'repo-1'
);
assert.equal(findRepoBranchStatus([status('repo-1')], 'repo-2'), undefined);

console.log('gitPanelState contract: ok');
