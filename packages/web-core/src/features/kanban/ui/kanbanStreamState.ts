export type KanbanStreamState = 'loading' | 'unavailable' | 'ready';

export function deriveKanbanStreamState({
  isLoading,
  hasError,
}: {
  isLoading: boolean;
  hasError: boolean;
}): KanbanStreamState {
  if (isLoading) return 'loading';
  return hasError ? 'unavailable' : 'ready';
}
