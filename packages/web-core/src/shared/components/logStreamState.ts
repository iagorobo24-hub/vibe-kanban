export type LogStreamViewState = 'empty' | 'error' | 'logs' | 'logs-with-error';

export function deriveLogStreamViewState(
  logCount: number,
  hasError: boolean
): LogStreamViewState {
  if (hasError) return logCount > 0 ? 'logs-with-error' : 'error';
  return logCount > 0 ? 'logs' : 'empty';
}
