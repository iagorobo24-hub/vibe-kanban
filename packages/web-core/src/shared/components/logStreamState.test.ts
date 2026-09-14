import assert from 'node:assert/strict';
import {
  deriveLogStreamViewState,
  type LogStreamViewState,
} from './logStreamState';

const cases: Array<[number, boolean, boolean, LogStreamViewState]> = [
  [0, false, false, 'empty'],
  [0, true, false, 'error'],
  [3, false, false, 'logs'],
  [3, true, false, 'logs-with-error'],
  [0, false, true, 'loading'],
  [3, false, true, 'logs-loading'],
];

for (const [logCount, hasError, isLoading, expected] of cases) {
  assert.equal(
    deriveLogStreamViewState(logCount, hasError, isLoading),
    expected
  );
}

console.log('logStreamState contract: ok');
