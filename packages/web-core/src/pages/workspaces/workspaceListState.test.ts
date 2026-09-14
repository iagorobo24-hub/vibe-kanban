import { getWorkspaceListViewState } from './workspaceListState';

function expectState(
  input: Parameters<typeof getWorkspaceListViewState>[0],
  expected: ReturnType<typeof getWorkspaceListViewState>
) {
  const actual = getWorkspaceListViewState(input);
  if (actual !== expected) {
    throw new Error(`Expected ${expected}, received ${actual}`);
  }
}

expectState(
  { activeCount: 0, archivedCount: 0, isLoading: true, hasError: false },
  'loading'
);
expectState(
  { activeCount: 0, archivedCount: 0, isLoading: true, hasError: true },
  'error'
);
expectState(
  { activeCount: 0, archivedCount: 0, isLoading: false, hasError: false },
  'empty'
);
expectState(
  { activeCount: 0, archivedCount: 2, isLoading: false, hasError: true },
  'workspaces-with-error'
);
expectState(
  { activeCount: 3, archivedCount: 0, isLoading: false, hasError: false },
  'workspaces'
);

console.log('workspaceListState contract: ok');
