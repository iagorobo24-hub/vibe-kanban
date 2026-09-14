import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
} from 'react';
import { useTranslation } from 'react-i18next';
import type { Workspace, Session, RepoWithTargetBranch } from 'shared/types';
import { WarningCircleIcon } from '@phosphor-icons/react';
import { createWorkspaceWithSession } from '@/shared/types/attempt';
import { WorkspacesMain } from '@vibe/ui/components/WorkspacesMain';
import {
  ConversationList,
  type ConversationListHandle,
} from '@/features/workspace-chat/ui/ConversationListContainer';
import { SessionChatBoxContainer } from '@/features/workspace-chat/ui/SessionChatBoxContainer';
import { ContextBarContainer } from './ContextBarContainer';
import { EntriesProvider } from '@/features/workspace-chat/model/contexts/EntriesContext';
import { MessageEditProvider } from '@/features/workspace-chat/model/contexts/MessageEditContext';
import { RetryUiProvider } from '@/features/workspace-chat/model/contexts/RetryUiContext';
import { ApprovalFeedbackProvider } from '@/features/workspace-chat/model/contexts/ApprovalFeedbackContext';
import { forwardWheelToScroller } from '@/features/workspace-chat/ui/forwardWheelToScroller';
import { useDiffStats } from '@/shared/stores/useWorkspaceDiffStore';
import { getSessionListViewState } from '@/features/workspace-chat/ui/sessionListState';

function SessionStreamNotice({
  stale,
  onRetry,
}: {
  stale: boolean;
  onRetry?: () => void;
}) {
  const { t } = useTranslation('common');

  return (
    <div
      className="mb-base flex w-chat max-w-full items-start gap-half rounded-md border border-error/30 bg-error/5 px-base py-half text-sm"
      role={stale ? 'status' : 'alert'}
    >
      <WarningCircleIcon
        className="mt-0.5 size-icon-sm shrink-0 text-error"
        aria-hidden="true"
      />
      <div className="min-w-0 flex-1">
        <p className="text-high">
          {t(
            stale
              ? 'workspaces.sessionsMayBeStale'
              : 'workspaces.sessionsUnavailable'
          )}
        </p>
        {onRetry && (
          <button
            type="button"
            onClick={onRetry}
            className="mt-half text-xs font-medium text-brand hover:underline focus:outline-none focus-visible:ring-2 focus-visible:ring-brand focus-visible:ring-offset-2"
          >
            {t('workspaces.retrySessions')}
          </button>
        )}
      </div>
    </div>
  );
}

/**
 * Isolated component that reads diffStats from WorkspaceContext.
 * By pushing the context subscription down to this leaf, the parent
 * WorkspacesMainContainer (and its ConversationList child) no longer
 * rerenders when diffs/comments/repos stream in.
 */
function ChatBoxWithDiffStats({
  session,
  workspaceId,
  isNewSessionMode,
  sessions,
  isSessionsLoading,
  sessionsError,
  onRetrySessions,
  onSelectSession,
  onStartNewSession,
  onScrollToPreviousMessage,
  onScrollToBottom,
  onScrollToUserMessage,
  getActiveTurnPatchKey,
}: {
  session: Session | undefined;
  workspaceId: string | undefined;
  isNewSessionMode: boolean;
  sessions: Session[];
  isSessionsLoading: boolean;
  sessionsError: string | null;
  onRetrySessions: () => void;
  onSelectSession: (sessionId: string) => void;
  onStartNewSession: () => void;
  onScrollToPreviousMessage: () => void;
  onScrollToBottom: (behavior?: 'auto' | 'smooth') => void;
  onScrollToUserMessage: (patchKey: string) => void;
  getActiveTurnPatchKey: () => string | null;
}) {
  const diffStats = useDiffStats();
  const sessionListState = getSessionListViewState({
    sessionCount: sessions.length,
    isLoading: isSessionsLoading,
    hasError: Boolean(sessionsError),
  });
  const sessionDataUnavailable =
    sessionListState === 'loading' || sessionListState === 'error';
  const effectiveIsNewSessionMode = sessionDataUnavailable
    ? false
    : isNewSessionMode;

  return (
    <div className="flex w-full flex-col items-center">
      {(sessionListState === 'loading' || sessionListState === 'error') && (
        <SessionStreamNotice
          stale={false}
          onRetry={sessionListState === 'error' ? onRetrySessions : undefined}
        />
      )}
      {sessionListState === 'sessions-with-error' && (
        <SessionStreamNotice stale onRetry={onRetrySessions} />
      )}
      <SessionChatBoxContainer
        {...(effectiveIsNewSessionMode && workspaceId
          ? {
              mode: 'new-session' as const,
              workspaceId,
              onSelectSession,
            }
          : session
            ? {
                mode: 'existing-session' as const,
                session,
                onSelectSession,
                onStartNewSession,
              }
            : {
                mode: 'placeholder' as const,
              })}
        sessions={sessions}
        filesChanged={diffStats.files_changed}
        linesAdded={diffStats.lines_added}
        linesRemoved={diffStats.lines_removed}
        disableViewCode={false}
        showOpenWorkspaceButton={false}
        onScrollToPreviousMessage={onScrollToPreviousMessage}
        onScrollToBottom={onScrollToBottom}
        onScrollToUserMessage={onScrollToUserMessage}
        getActiveTurnPatchKey={getActiveTurnPatchKey}
      />
    </div>
  );
}

export interface WorkspacesMainContainerHandle {
  scrollToBottom: (behavior?: 'auto' | 'smooth') => void;
}

interface WorkspacesMainContainerProps {
  selectedWorkspace: Workspace | null;
  selectedSession: Session | undefined;
  selectedSessionId: string | undefined;
  sessions: Session[];
  repos: RepoWithTargetBranch[];
  onSelectSession: (sessionId: string) => void;
  isLoading: boolean;
  isSessionsLoading: boolean;
  sessionsError: string | null;
  onRetrySessions: () => void;
  isNewSessionMode: boolean;
  onStartNewSession: () => void;
}

export const WorkspacesMainContainer = forwardRef<
  WorkspacesMainContainerHandle,
  WorkspacesMainContainerProps
>(function WorkspacesMainContainer(
  {
    selectedWorkspace,
    selectedSession,
    selectedSessionId,
    sessions,
    repos,
    onSelectSession,
    isLoading,
    isSessionsLoading,
    sessionsError,
    onRetrySessions,
    isNewSessionMode,
    onStartNewSession,
  },
  ref
) {
  const containerRef = useRef<HTMLElement>(null);
  const conversationListRef = useRef<ConversationListHandle>(null);

  const workspaceWithSession = useMemo(() => {
    if (!selectedWorkspace) return undefined;
    return createWorkspaceWithSession(selectedWorkspace, selectedSession);
  }, [selectedWorkspace, selectedSession]);

  const handleScrollToPreviousMessage = useCallback(() => {
    conversationListRef.current?.scrollToPreviousUserMessage();
  }, []);

  const handleScrollToUserMessage = useCallback((patchKey: string) => {
    conversationListRef.current?.scrollToEntryByPatchKey(patchKey);
  }, []);

  const handleGetActiveTurnPatchKey = useCallback(() => {
    return conversationListRef.current?.getVisibleUserMessagePatchKey() ?? null;
  }, []);

  const [isAtBottom, setIsAtBottom] = useState(true);
  const isAtBottomRef = useRef(isAtBottom);
  const handleAtBottomChange = useCallback((atBottom: boolean) => {
    isAtBottomRef.current = atBottom;
    setIsAtBottom(atBottom);
  }, []);

  const handleScrollToBottom = useCallback(
    (behavior: 'auto' | 'smooth' = 'smooth') => {
      conversationListRef.current?.scrollToBottom(behavior);
    },
    []
  );

  const { session } = workspaceWithSession ?? {};

  useEffect(() => {
    isAtBottomRef.current = isAtBottom;
  }, [isAtBottom]);

  useEffect(() => {
    const container = containerRef.current;
    if (!container || typeof ResizeObserver === 'undefined') return;

    const chatBoxContainer = container.querySelector<HTMLElement>(
      '[data-chatbox-container="true"]'
    );
    if (!chatBoxContainer) return;

    let previousHeight = chatBoxContainer.getBoundingClientRect().height;

    const observer = new ResizeObserver((entries) => {
      const nextHeight =
        entries[0]?.contentRect.height ??
        chatBoxContainer.getBoundingClientRect().height;

      if (Math.abs(nextHeight - previousHeight) < 0.5) return;
      const heightDelta = nextHeight - previousHeight;
      previousHeight = nextHeight;

      if (!isAtBottomRef.current) return;

      requestAnimationFrame(() => {
        if (!isAtBottomRef.current) return;
        conversationListRef.current?.adjustScrollBy(heightDelta);
      });
    });

    observer.observe(chatBoxContainer);

    return () => {
      observer.disconnect();
    };
  }, [workspaceWithSession?.id, session?.id]);

  const entriesProviderKey = workspaceWithSession
    ? `${workspaceWithSession.id}-${selectedSessionId ?? 'new'}`
    : 'empty';

  const conversationContent = workspaceWithSession ? (
    <div
      className="agentos-workspaces-main__conversation flex-1 min-h-0 overflow-hidden flex justify-center"
      onWheel={(e) => forwardWheelToScroller(e, conversationListRef)}
    >
      <div className="w-chat max-w-full h-full">
        <RetryUiProvider workspaceId={workspaceWithSession.id}>
          <ConversationList
            key={entriesProviderKey}
            ref={conversationListRef}
            attempt={workspaceWithSession}
            repos={repos}
            onAtBottomChange={handleAtBottomChange}
            sessionScopeId={selectedSessionId}
          />
        </RetryUiProvider>
      </div>
    </div>
  ) : null;

  const chatBoxContent = (
    <ChatBoxWithDiffStats
      session={session}
      workspaceId={workspaceWithSession?.id}
      isNewSessionMode={isNewSessionMode}
      sessions={sessions}
      isSessionsLoading={isSessionsLoading}
      sessionsError={sessionsError}
      onRetrySessions={onRetrySessions}
      onSelectSession={onSelectSession}
      onStartNewSession={onStartNewSession}
      onScrollToPreviousMessage={handleScrollToPreviousMessage}
      onScrollToBottom={handleScrollToBottom}
      onScrollToUserMessage={handleScrollToUserMessage}
      getActiveTurnPatchKey={handleGetActiveTurnPatchKey}
    />
  );

  const contextBarContent = workspaceWithSession ? (
    <ContextBarContainer containerRef={containerRef} />
  ) : null;

  useImperativeHandle(
    ref,
    () => ({
      scrollToBottom: (behavior = 'smooth') => {
        conversationListRef.current?.scrollToBottom(behavior);
      },
    }),
    []
  );

  return (
    <ApprovalFeedbackProvider>
      <EntriesProvider key={entriesProviderKey}>
        <MessageEditProvider>
          <WorkspacesMain
            workspaceWithSession={
              workspaceWithSession ? { id: workspaceWithSession.id } : undefined
            }
            isLoading={isLoading}
            containerRef={containerRef}
            conversationContent={conversationContent}
            chatBoxContent={chatBoxContent}
            contextBarContent={contextBarContent}
            isAtBottom={isAtBottom}
            onAtBottomChange={handleAtBottomChange}
            onScrollToBottom={handleScrollToBottom}
          />
        </MessageEditProvider>
      </EntriesProvider>
    </ApprovalFeedbackProvider>
  );
});
