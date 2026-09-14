import { useCallback, useMemo } from 'react';
import type { ApprovalInfo } from 'shared/types';
import { useJsonPatchWsStream } from './useJsonPatchWsStream';
import {
  AGENTOS_APPROVAL_FIXTURE_INFO,
  isAgentOSQaFixtureEnabled,
} from '@/shared/lib/agentOSQaFixtures';

interface UseApprovalsResult {
  pendingApprovals: ApprovalInfo[];
  getPendingForProcess: (executionProcessId: string) => ApprovalInfo | null;
  getPendingById: (approvalId: string) => ApprovalInfo | null;
  isConnected: boolean;
}

type ApprovalState = {
  pending: Record<string, ApprovalInfo>;
};

export function useApprovals(): UseApprovalsResult {
  const approvalFixtureEnabled = isAgentOSQaFixtureEnabled('approval');
  const { data, isConnected } = useJsonPatchWsStream<ApprovalState>(
    '/api/approvals/stream/ws',
    !approvalFixtureEnabled,
    () => ({ pending: {} })
  );

  const pendingById = useMemo(
    () =>
      approvalFixtureEnabled
        ? {
            [AGENTOS_APPROVAL_FIXTURE_INFO.approval_id]:
              AGENTOS_APPROVAL_FIXTURE_INFO,
          }
        : (data?.pending ?? {}),
    [approvalFixtureEnabled, data?.pending]
  );
  const pendingApprovals = useMemo(
    () => Object.values(pendingById),
    [pendingById]
  );

  const getPendingForProcess = useCallback(
    (executionProcessId: string): ApprovalInfo | null => {
      for (const info of pendingApprovals) {
        if (info.execution_process_id === executionProcessId) {
          return info;
        }
      }
      return null;
    },
    [pendingApprovals]
  );

  const getPendingById = useCallback(
    (approvalId: string): ApprovalInfo | null => {
      return pendingById[approvalId] ?? null;
    },
    [pendingById]
  );

  return {
    pendingApprovals,
    getPendingForProcess,
    getPendingById,
    isConnected: approvalFixtureEnabled || isConnected,
  };
}
