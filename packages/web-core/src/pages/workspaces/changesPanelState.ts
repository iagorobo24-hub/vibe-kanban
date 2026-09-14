export type ChangesPanelViewState =
  | 'loading'
  | 'empty'
  | 'diffs'
  | 'error'
  | 'diffs-with-error';

export interface ChangesPanelStateInput {
  diffCount: number;
  isInitialized: boolean;
  hasError: boolean;
}

export function deriveChangesPanelViewState({
  diffCount,
  isInitialized,
  hasError,
}: ChangesPanelStateInput): ChangesPanelViewState {
  if (!isInitialized && !hasError) return 'loading';
  if (hasError) return diffCount > 0 ? 'diffs-with-error' : 'error';
  return diffCount > 0 ? 'diffs' : 'empty';
}
