import { useSyncExternalStore } from 'react';
import {
  BaseCodingAgent,
  ExecutionProcessStatus,
  PermissionPolicy,
  type ApprovalInfo,
  type ApprovalStatus,
  type ExecutionProcess,
} from 'shared/types';

export type AgentOSQaFixture = 'stream-error' | 'tool-stream' | 'approval';

export const AGENTOS_TOOL_FIXTURE_PROCESS_ID =
  'agentos-fixture-tool-stream-process';
export const AGENTOS_TOOL_FIXTURE_SESSION_ID =
  'agentos-fixture-tool-stream-session';
export const AGENTOS_APPROVAL_FIXTURE_ID = 'agentos-fixture-approval';
const AGENTOS_QA_FIXTURE_TIMESTAMP = '2026-09-14T10:00:00.000Z';

type AgentOSQaApprovalResponse =
  | Extract<ApprovalStatus, { status: 'approved' }>
  | Extract<ApprovalStatus, { status: 'denied' }>
  | null;

let agentOSQaApprovalResponse: AgentOSQaApprovalResponse = null;
const agentOSQaApprovalListeners = new Set<() => void>();

export const AGENTOS_APPROVAL_FIXTURE_INFO: ApprovalInfo = {
  approval_id: AGENTOS_APPROVAL_FIXTURE_ID,
  tool_name: 'shell',
  execution_process_id: AGENTOS_TOOL_FIXTURE_PROCESS_ID,
  is_question: false,
  created_at: '2026-09-14T10:00:04.000Z',
  timeout_at: '2099-09-14T10:00:04.000Z',
};

export function createAgentOSQaFixtureProcess(
  status: 'running' | 'completed'
): ExecutionProcess {
  const isRunning = status === 'running';

  return {
    id: AGENTOS_TOOL_FIXTURE_PROCESS_ID,
    session_id: AGENTOS_TOOL_FIXTURE_SESSION_ID,
    run_reason: 'codingagent',
    executor_action: {
      typ: {
        type: 'CodingAgentInitialRequest',
        prompt: 'Revisa el flujo de autenticación y prepara un resumen.',
        executor_config: {
          executor: BaseCodingAgent.OPENCODE,
          variant: 'Build',
          model_id: 'opencode/zen-free',
          agent_id: null,
          reasoning_id: 'high',
          permission_policy: PermissionPolicy.AUTO,
        },
        working_dir: null,
      },
      next_action: null,
    },
    status: isRunning
      ? ExecutionProcessStatus.running
      : ExecutionProcessStatus.completed,
    exit_code: isRunning ? null : 0n,
    dropped: false,
    started_at: AGENTOS_QA_FIXTURE_TIMESTAMP,
    completed_at: isRunning ? null : '2026-09-14T10:00:08.000Z',
    created_at: AGENTOS_QA_FIXTURE_TIMESTAMP,
    updated_at: '2026-09-14T10:00:08.000Z',
  };
}

export function getAgentOSQaApprovalResponse(): AgentOSQaApprovalResponse {
  return agentOSQaApprovalResponse;
}

export function subscribeToAgentOSQaApprovalResponse(
  listener: () => void
): () => void {
  agentOSQaApprovalListeners.add(listener);
  return () => agentOSQaApprovalListeners.delete(listener);
}

export function resolveAgentOSQaApproval(
  response: Exclude<AgentOSQaApprovalResponse, null>
): void {
  agentOSQaApprovalResponse = response;
  for (const listener of agentOSQaApprovalListeners) listener();
}

export function useAgentOSQaApprovalResponse(): AgentOSQaApprovalResponse {
  return useSyncExternalStore(
    subscribeToAgentOSQaApprovalResponse,
    getAgentOSQaApprovalResponse,
    getAgentOSQaApprovalResponse
  );
}

export function getAgentOSQaFixture(): AgentOSQaFixture | null {
  if (!import.meta.env.DEV || typeof window === 'undefined') {
    return null;
  }

  const value = new URLSearchParams(window.location.search).get(
    'agentosFixture'
  );

  if (
    value === 'stream-error' ||
    value === 'tool-stream' ||
    value === 'approval'
  ) {
    return value;
  }

  return null;
}

export function isAgentOSQaFixtureEnabled(fixture: AgentOSQaFixture): boolean {
  return getAgentOSQaFixture() === fixture;
}
