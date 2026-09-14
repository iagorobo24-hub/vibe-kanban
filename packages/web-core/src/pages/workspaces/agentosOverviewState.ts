export type AgentOSOverviewConnection = 'connecting' | 'online' | 'offline';

export interface AgentOSOverviewStreamInput {
  isLoading: boolean;
  isConnected: boolean;
  error: string | null;
}

export interface AgentOSOverviewStreamState {
  connection: AgentOSOverviewConnection;
  metricsUnavailable: boolean;
  streamUnavailable: boolean;
}

export function deriveAgentOSOverviewStreamState({
  isLoading,
  isConnected,
  error,
}: AgentOSOverviewStreamInput): AgentOSOverviewStreamState {
  const streamUnavailable = !isLoading && (!isConnected || Boolean(error));

  return {
    connection: isLoading
      ? 'connecting'
      : isConnected && !error
        ? 'online'
        : 'offline',
    metricsUnavailable: isLoading || streamUnavailable,
    streamUnavailable,
  };
}
