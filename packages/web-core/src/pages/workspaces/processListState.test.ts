import assert from 'node:assert/strict';
import {
  deriveProcessListViewState,
  type ProcessListViewState,
} from './processListState';

function expectState(
  input: Parameters<typeof deriveProcessListViewState>[0],
  expected: ProcessListViewState
) {
  assert.equal(deriveProcessListViewState(input), expected);
}

expectState({ processCount: 0, isLoading: true, hasError: false }, 'loading');
expectState({ processCount: 0, isLoading: false, hasError: true }, 'error');
expectState({ processCount: 0, isLoading: false, hasError: false }, 'empty');
expectState(
  { processCount: 2, isLoading: false, hasError: false },
  'processes'
);
expectState(
  { processCount: 2, isLoading: false, hasError: true },
  'processes-with-error'
);

console.log('processListState contract: ok');
