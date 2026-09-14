import { strict as assert } from 'node:assert';
import { deriveProjectDataState } from './projectDataState';

assert.equal(
  deriveProjectDataState({
    isLoading: true,
    hasError: false,
    hasProject: false,
  }),
  'loading'
);

assert.equal(
  deriveProjectDataState({
    isLoading: false,
    hasError: true,
    hasProject: false,
  }),
  'unavailable'
);

assert.equal(
  deriveProjectDataState({
    isLoading: false,
    hasError: false,
    hasProject: true,
  }),
  'ready'
);

assert.equal(
  deriveProjectDataState({
    isLoading: false,
    hasError: false,
    hasProject: false,
  }),
  'missing'
);

console.log('projectDataState contract: ok');
