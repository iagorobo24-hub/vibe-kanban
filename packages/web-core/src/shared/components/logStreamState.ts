export type LogStreamViewState =
  | 'empty'
  | 'error'
  | 'loading'
  | 'logs'
  | 'logs-loading'
  | 'logs-with-error';

export function deriveLogStreamViewState(
  logCount: number,
  hasError: boolean,
  isLoading = false
): LogStreamViewState {
  if (hasError) return logCount > 0 ? 'logs-with-error' : 'error';
  if (isLoading) return logCount > 0 ? 'logs-loading' : 'loading';
  return logCount > 0 ? 'logs' : 'empty';
}
