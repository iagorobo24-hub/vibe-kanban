export type WorkspaceListViewState =
  | 'loading'
  | 'empty'
  | 'workspaces'
  | 'error'
  | 'workspaces-with-error';

interface WorkspaceListStateInput {
  activeCount: number;
  archivedCount: number;
  isLoading: boolean;
  hasError: boolean;
}

export function getWorkspaceListViewState({
  activeCount,
  archivedCount,
  isLoading,
  hasError,
}: WorkspaceListStateInput): WorkspaceListViewState {
  const totalCount = activeCount + archivedCount;

  if (hasError) {
    return totalCount > 0 ? 'workspaces-with-error' : 'error';
  }

  if (isLoading) return 'loading';
  return totalCount > 0 ? 'workspaces' : 'empty';
}
