import assert from 'node:assert/strict';
import {
  deriveLogStreamViewState,
  type LogStreamViewState,
} from './logStreamState';

const cases: Array<[number, boolean, LogStreamViewState]> = [
  [0, false, 'empty'],
  [0, true, 'error'],
  [3, false, 'logs'],
  [3, true, 'logs-with-error'],
];

for (const [logCount, hasError, expected] of cases) {
  assert.equal(deriveLogStreamViewState(logCount, hasError), expected);
}

console.log('logStreamState contract: ok');
