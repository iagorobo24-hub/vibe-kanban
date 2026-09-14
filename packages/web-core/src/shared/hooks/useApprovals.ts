import { useCallback, useMemo } from 'react';
import type { ApprovalInfo } from 'shared/types';
import { useJsonPatchWsStream } from './useJsonPatchWsStream';
import {
  AGENTOS_APPROVAL_FIXTURE_INFO,
  isAgentOSQaFixtureEnabled,
  useAgentOSQaApprovalResponse,
} from '@/shared/lib/agentOSQaFixtures';

interface UseApprovalsResult {
  pendingApprovals: ApprovalInfo[];
  getPendingForProcess: (executionProcessId: string) => ApprovalInfo | null;
  getPendingById: (approvalId: string) => ApprovalInfo | null;
  isConnected: boolean;
  isLoading: boolean;
  error: string | null;
  retry: () => void;
}

type ApprovalState = {
  pending: Record<string, ApprovalInfo>;
};

export function useApprovals(): UseApprovalsResult {
  const approvalFixtureEnabled = isAgentOSQaFixtureEnabled('approval');
  const approvalFixtureResponse = useAgentOSQaApprovalResponse();
  const { data, isConnected, isInitialized, error, retry } =
    useJsonPatchWsStream<ApprovalState>(
      '/api/approvals/stream/ws',
      !approvalFixtureEnabled,
      () => ({ pending: {} })
    );

  const pendingById = useMemo(
    () =>
      approvalFixtureEnabled && !approvalFixtureResponse
        ? {
            [AGENTOS_APPROVAL_FIXTURE_INFO.approval_id]:
              AGENTOS_APPROVAL_FIXTURE_INFO,
          }
        : (data?.pending ?? {}),
    [approvalFixtureEnabled, approvalFixtureResponse, data?.pending]
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
    isLoading: !approvalFixtureEnabled && !isInitialized && !error,
    error,
    retry,
  };
}
