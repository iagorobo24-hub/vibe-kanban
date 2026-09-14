export type ProjectDataState =
  | 'loading'
  | 'unavailable'
  | 'ready'
  | 'missing';

export function deriveProjectDataState({
  isLoading,
  hasError,
  hasProject,
}: {
  isLoading: boolean;
  hasError: boolean;
  hasProject: boolean;
}): ProjectDataState {
  if (isLoading) return 'loading';
  if (hasError) return 'unavailable';
  return hasProject ? 'ready' : 'missing';
}
