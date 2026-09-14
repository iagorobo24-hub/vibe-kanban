import type { ApprovalInfo } from 'shared/types';

export type AgentOSQaFixture = 'stream-error' | 'tool-stream' | 'approval';

export const AGENTOS_TOOL_FIXTURE_PROCESS_ID =
  'agentos-fixture-tool-stream-process';
export const AGENTOS_TOOL_FIXTURE_SESSION_ID =
  'agentos-fixture-tool-stream-session';
export const AGENTOS_APPROVAL_FIXTURE_ID = 'agentos-fixture-approval';

export const AGENTOS_APPROVAL_FIXTURE_INFO: ApprovalInfo = {
  approval_id: AGENTOS_APPROVAL_FIXTURE_ID,
  tool_name: 'shell',
  execution_process_id: AGENTOS_TOOL_FIXTURE_PROCESS_ID,
  is_question: false,
  created_at: '2026-09-14T10:00:04.000Z',
  timeout_at: '2099-09-14T10:00:04.000Z',
};

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
