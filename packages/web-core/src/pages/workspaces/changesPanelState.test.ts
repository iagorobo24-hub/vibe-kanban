import assert from 'node:assert/strict';
import {
  deriveChangesPanelViewState,
  type ChangesPanelViewState,
} from './changesPanelState';

function expectState(
  input: Parameters<typeof deriveChangesPanelViewState>[0],
  expected: ChangesPanelViewState
) {
  assert.equal(deriveChangesPanelViewState(input), expected);
}

expectState({ diffCount: 0, isInitialized: false, hasError: false }, 'loading');
expectState({ diffCount: 0, isInitialized: false, hasError: true }, 'error');
expectState({ diffCount: 0, isInitialized: true, hasError: false }, 'empty');
expectState({ diffCount: 0, isInitialized: true, hasError: true }, 'error');
expectState({ diffCount: 2, isInitialized: true, hasError: false }, 'diffs');
expectState(
  { diffCount: 2, isInitialized: true, hasError: true },
  'diffs-with-error'
);

console.log('changesPanelState contract: ok');
