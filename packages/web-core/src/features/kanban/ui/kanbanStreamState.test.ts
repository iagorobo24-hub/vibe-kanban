import { strict as assert } from 'node:assert';
import { deriveKanbanStreamState } from './kanbanStreamState';

assert.equal(
  deriveKanbanStreamState({ isLoading: true, hasError: false }),
  'loading'
);
assert.equal(
  deriveKanbanStreamState({ isLoading: false, hasError: true }),
  'unavailable'
);
assert.equal(
  deriveKanbanStreamState({ isLoading: false, hasError: false }),
  'ready'
);

console.log('kanbanStreamState contract: ok');
