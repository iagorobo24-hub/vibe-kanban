export type SessionListViewState =
  | 'loading'
  | 'ready-empty'
  | 'sessions'
  | 'error'
  | 'sessions-with-error';

interface SessionListStateInput {
  sessionCount: number;
  isLoading: boolean;
  hasError: boolean;
}

export function getSessionListViewState({
  sessionCount,
  isLoading,
  hasError,
}: SessionListStateInput): SessionListViewState {
  if (hasError) {
    return sessionCount > 0 ? 'sessions-with-error' : 'error';
  }

  if (isLoading) return 'loading';
  return sessionCount > 0 ? 'sessions' : 'ready-empty';
}
