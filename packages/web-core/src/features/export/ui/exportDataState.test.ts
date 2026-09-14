import { strict as assert } from 'node:assert';
import { deriveExportDataState } from './exportDataState';

assert.equal(
  deriveExportDataState({ isLoading: true, hasError: false, itemCount: 0 }),
  'loading'
);
assert.equal(
  deriveExportDataState({ isLoading: false, hasError: true, itemCount: 0 }),
  'unavailable'
);
assert.equal(
  deriveExportDataState({ isLoading: false, hasError: true, itemCount: 2 }),
  'stale'
);
assert.equal(
  deriveExportDataState({ isLoading: false, hasError: false, itemCount: 0 }),
  'empty'
);
assert.equal(
  deriveExportDataState({ isLoading: false, hasError: false, itemCount: 2 }),
  'ready'
);

console.log('exportDataState contract: ok');
