import type { RepoBranchStatus } from 'shared/types';

export type GitPanelStatus = 'loading' | 'ready' | 'unavailable';

interface DeriveGitPanelStatusInput {
  repoIds: string[];
  branchStatus: RepoBranchStatus[] | undefined;
  isLoading: boolean;
  hasError: boolean;
}

export function deriveGitPanelStatus({
  repoIds,
  branchStatus,
  isLoading,
  hasError,
}: DeriveGitPanelStatusInput): GitPanelStatus {
  if (repoIds.length === 0) return 'ready';
  if (hasError) return 'unavailable';
  if (!branchStatus) return isLoading ? 'loading' : 'unavailable';

  const hasMissingRepoStatus = repoIds.some(
    (repoId) => !branchStatus.some((status) => status.repo_id === repoId)
  );

  return hasMissingRepoStatus ? 'unavailable' : 'ready';
}

export function findRepoBranchStatus(
  branchStatus: RepoBranchStatus[] | undefined,
  repoId: string
): RepoBranchStatus | undefined {
  return branchStatus?.find((status) => status.repo_id === repoId);
}
