import { getSessionListViewState } from './sessionListState';

function expectState(
  input: Parameters<typeof getSessionListViewState>[0],
  expected: ReturnType<typeof getSessionListViewState>
) {
  const actual = getSessionListViewState(input);
  if (actual !== expected) {
    throw new Error(`Expected ${expected}, received ${actual}`);
  }
}

expectState({ sessionCount: 0, isLoading: true, hasError: false }, 'loading');
expectState({ sessionCount: 0, isLoading: true, hasError: true }, 'error');
expectState(
  { sessionCount: 0, isLoading: false, hasError: false },
  'ready-empty'
);
expectState(
  { sessionCount: 2, isLoading: false, hasError: true },
  'sessions-with-error'
);
expectState({ sessionCount: 1, isLoading: false, hasError: false }, 'sessions');

console.log('sessionListState contract: ok');
