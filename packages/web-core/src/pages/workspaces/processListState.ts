export type ProcessListViewState =
  | 'loading'
  | 'empty'
  | 'processes'
  | 'error'
  | 'processes-with-error';

export interface ProcessListStateInput {
  processCount: number;
  isLoading: boolean;
  hasError: boolean;
}

export function deriveProcessListViewState({
  processCount,
  isLoading,
  hasError,
}: ProcessListStateInput): ProcessListViewState {
  if (isLoading && !hasError) return 'loading';
  if (hasError) return processCount > 0 ? 'processes-with-error' : 'error';
  return processCount > 0 ? 'processes' : 'empty';
}
