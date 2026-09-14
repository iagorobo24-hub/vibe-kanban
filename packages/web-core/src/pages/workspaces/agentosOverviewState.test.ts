import assert from 'node:assert/strict';

import { deriveAgentOSOverviewStreamState } from './agentosOverviewState';

assert.deepEqual(
  deriveAgentOSOverviewStreamState({
    isLoading: true,
    isConnected: false,
    error: null,
  }),
  {
    connection: 'connecting',
    metricsUnavailable: true,
    streamUnavailable: false,
  }
);

assert.deepEqual(
  deriveAgentOSOverviewStreamState({
    isLoading: false,
    isConnected: true,
    error: 'stale stream',
  }),
  {
    connection: 'offline',
    metricsUnavailable: true,
    streamUnavailable: true,
  }
);

assert.deepEqual(
  deriveAgentOSOverviewStreamState({
    isLoading: false,
    isConnected: true,
    error: null,
  }),
  {
    connection: 'online',
    metricsUnavailable: false,
    streamUnavailable: false,
  }
);

console.log('agentosOverviewState contract: ok');
