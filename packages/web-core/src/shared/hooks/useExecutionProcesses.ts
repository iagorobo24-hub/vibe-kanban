import { useCallback } from 'react';
import { useJsonPatchWsStream } from '@/shared/hooks/useJsonPatchWsStream';
import { useHostId } from '@/shared/providers/HostIdProvider';
import type { ExecutionProcess } from 'shared/types';
import {
  createAgentOSQaFixtureProcess,
  isAgentOSQaFixtureEnabled,
  useAgentOSQaApprovalResponse,
} from '@/shared/lib/agentOSQaFixtures';

type ExecutionProcessState = {
  execution_processes: Record<string, ExecutionProcess>;
};

interface UseExecutionProcessesResult {
  executionProcesses: ExecutionProcess[];
  executionProcessesById: Record<string, ExecutionProcess>;
  isAttemptRunning: boolean;
  isLoading: boolean;
  isConnected: boolean;
  error: string | null;
}

/**
 * Stream execution processes for a session via WebSocket (JSON Patch) and expose as array + map.
 * Server sends initial snapshot: replace /execution_processes with an object keyed by id.
 * Live updates arrive at /execution_processes/<id> via add/replace/remove operations.
 */
export const useExecutionProcesses = (
  sessionId: string | undefined,
  opts?: { showSoftDeleted?: boolean }
): UseExecutionProcessesResult => {
  const hostId = useHostId();
  const showSoftDeleted = opts?.showSoftDeleted;
  const approvalFixtureEnabled = isAgentOSQaFixtureEnabled('approval');
  const approvalFixtureResponse = useAgentOSQaApprovalResponse();
  let endpoint: string | undefined;

  if (sessionId && !approvalFixtureEnabled) {
    const apiBasePath = hostId ? `/api/host/${hostId}` : '/api';
    const params = new URLSearchParams({ session_id: sessionId });
    if (typeof showSoftDeleted === 'boolean') {
      params.set('show_soft_deleted', String(showSoftDeleted));
    }
    endpoint = `${apiBasePath}/execution-processes/stream/session/ws?${params.toString()}`;
  }

  const initialData = useCallback(
    (): ExecutionProcessState => ({ execution_processes: {} }),
    []
  );

  const { data, isConnected, isInitialized, error } =
    useJsonPatchWsStream<ExecutionProcessState>(
      endpoint,
      !!sessionId && !approvalFixtureEnabled,
      initialData
    );

  const streamedExecutionProcesses = Object.values(
    data?.execution_processes ?? {}
  ).sort(
    (a, b) =>
      new Date(a.created_at as unknown as string).getTime() -
      new Date(b.created_at as unknown as string).getTime()
  );

  // Guard against stale buffered stream data when switching sessions quickly.
  const fixtureExecutionProcesses =
    approvalFixtureEnabled && sessionId
      ? [
          createAgentOSQaFixtureProcess(
            approvalFixtureResponse?.status === 'denied'
              ? 'completed'
              : 'running'
          ),
        ]
      : [];

  const executionProcesses = approvalFixtureEnabled
    ? fixtureExecutionProcesses
    : sessionId
      ? streamedExecutionProcesses.filter(
          (executionProcess) => executionProcess.session_id === sessionId
        )
      : streamedExecutionProcesses;

  const executionProcessesById = executionProcesses.reduce<
    Record<string, ExecutionProcess>
  >((processesById, executionProcess) => {
    processesById[executionProcess.id] = executionProcess;
    return processesById;
  }, {});

  const isAttemptRunning = executionProcesses.some(
    (process) =>
      (process.run_reason === 'codingagent' ||
        process.run_reason === 'setupscript' ||
        process.run_reason === 'cleanupscript' ||
        process.run_reason === 'archivescript') &&
      process.status === 'running'
  );
  const isLoading =
    !!sessionId && !approvalFixtureEnabled && !isInitialized && !error; // until first snapshot

  return {
    executionProcesses,
    executionProcessesById,
    isAttemptRunning,
    isLoading,
    isConnected: approvalFixtureEnabled || isConnected,
    error,
  };
};
