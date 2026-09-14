export type ExportDataState =
  | 'loading'
  | 'empty'
  | 'ready'
  | 'unavailable'
  | 'stale';

export function deriveExportDataState({
  isLoading,
  hasError,
  itemCount,
}: {
  isLoading: boolean;
  hasError: boolean;
  itemCount: number;
}): ExportDataState {
  if (isLoading) return 'loading';
  if (hasError) return itemCount > 0 ? 'stale' : 'unavailable';
  return itemCount > 0 ? 'ready' : 'empty';
}
