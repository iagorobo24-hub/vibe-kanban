import { useEffect, useMemo, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useExecutionProcessesContext } from '@/shared/hooks/useExecutionProcessesContext';
import { useLogsPanel } from '@/shared/hooks/useLogsPanel';
import { ProcessListItem } from '@vibe/ui/components/ProcessListItem';
import { InputField } from '@vibe/ui/components/InputField';
import {
  ArrowClockwiseIcon,
  CaretUpIcon,
  CaretDownIcon,
  SpinnerGapIcon,
  TerminalIcon,
  WarningCircleIcon,
} from '@phosphor-icons/react';
import { cn } from '@/shared/lib/utils';
import { PrimaryButton } from '@vibe/ui/components/PrimaryButton';
import {
  deriveProcessListViewState,
  type ProcessListViewState,
} from './processListState';

function ProcessListStatus({
  viewState,
  onRetry,
}: {
  viewState: Extract<ProcessListViewState, 'loading' | 'error'>;
  onRetry: () => void;
}) {
  const { t } = useTranslation(['common', 'tasks']);

  return (
    <div
      className="flex flex-col items-center justify-center gap-base px-base py-double text-center text-low"
      role={viewState === 'error' ? 'alert' : 'status'}
      aria-live="polite"
    >
      {viewState === 'loading' ? (
        <SpinnerGapIcon
          className="size-icon-lg animate-spin"
          aria-hidden="true"
        />
      ) : (
        <WarningCircleIcon
          className="size-icon-lg text-warning"
          weight="fill"
          aria-hidden="true"
        />
      )}
      <p className="font-medium text-normal">
        {viewState === 'loading'
          ? t('tasks:processes.loading')
          : t('tasks:processes.processesUnavailable')}
      </p>
      {viewState === 'error' && (
        <PrimaryButton
          variant="tertiary"
          actionIcon={ArrowClockwiseIcon}
          onClick={onRetry}
        >
          {t('tasks:processes.retryProcesses')}
        </PrimaryButton>
      )}
    </div>
  );
}

function ProcessListWarning({ onRetry }: { onRetry: () => void }) {
  const { t } = useTranslation('tasks');

  return (
    <div
      className="mx-half mb-half flex flex-wrap items-center justify-between gap-half rounded-sm border border-warning bg-warning/10 px-half py-quarter text-xs text-warning"
      role="alert"
    >
      <span className="flex min-w-0 items-center gap-half">
        <WarningCircleIcon
          className="size-icon-sm shrink-0"
          weight="fill"
          aria-hidden="true"
        />
        {t('processes.processesMayBeStale')}
      </span>
      <PrimaryButton
        variant="tertiary"
        actionIcon={ArrowClockwiseIcon}
        onClick={onRetry}
        className="shrink-0"
      >
        {t('processes.retryProcesses')}
      </PrimaryButton>
    </div>
  );
}

export function ProcessListContainer() {
  const {
    logsPanelContent,
    logSearchQuery: searchQuery,
    logMatchIndices,
    logCurrentMatchIdx: currentMatchIdx,
    setLogSearchQuery: onSearchQueryChange,
    handleLogPrevMatch: onPrevMatch,
    handleLogNextMatch: onNextMatch,
    viewProcessInPanel: onSelectProcess,
    expandTerminal,
    isTerminalExpanded,
  } = useLogsPanel();

  const selectedProcessId =
    logsPanelContent?.type === 'process' ? logsPanelContent.processId : null;
  const disableAutoSelect =
    logsPanelContent?.type === 'tool' || logsPanelContent?.type === 'terminal';
  const matchCount = logMatchIndices.length;
  const { t } = useTranslation(['common', 'tasks']);
  const { executionProcessesVisible, isLoading, error, retry } =
    useExecutionProcessesContext();
  const viewState = deriveProcessListViewState({
    processCount: executionProcessesVisible.length,
    isLoading,
    hasError: Boolean(error),
  });

  // Sort processes by created_at descending (newest first)
  const sortedProcesses = useMemo(() => {
    return [...executionProcessesVisible].sort((a, b) => {
      return (
        new Date(b.created_at).getTime() - new Date(a.created_at).getTime()
      );
    });
  }, [executionProcessesVisible]);

  // Auto-select latest process if none selected (unless disabled)
  useEffect(() => {
    if (
      !disableAutoSelect &&
      !selectedProcessId &&
      sortedProcesses.length > 0
    ) {
      onSelectProcess(sortedProcesses[0].id);
    }
  }, [disableAutoSelect, selectedProcessId, sortedProcesses, onSelectProcess]);

  const handleSelectProcess = useCallback(
    (processId: string) => {
      onSelectProcess(processId);
    },
    [onSelectProcess]
  );

  const handleSearchKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLDivElement>) => {
      if (e.key === 'Enter') {
        if (e.shiftKey) {
          onPrevMatch?.();
        } else {
          onNextMatch?.();
        }
      } else if (e.key === 'Escape') {
        onSearchQueryChange?.('');
      }
    },
    [onPrevMatch, onNextMatch, onSearchQueryChange]
  );

  const showSearch = onSearchQueryChange !== undefined;

  const searchBar = showSearch && (
    <div
      className="flex items-center gap-2 shrink-0 mx-base mb-base"
      onKeyDown={handleSearchKeyDown}
    >
      <InputField
        value={searchQuery}
        onChange={onSearchQueryChange}
        placeholder={t('logs.searchLogs')}
        variant="search"
        className="flex-1"
      />
      {searchQuery && (
        <>
          <span className="text-xs text-low whitespace-nowrap">
            {matchCount > 0
              ? t('search.matchCount', {
                  current: currentMatchIdx + 1,
                  total: matchCount,
                })
              : t('search.noMatches')}
          </span>
          <div className="flex items-center gap-1">
            <button
              onClick={onPrevMatch}
              disabled={matchCount === 0}
              className="rounded-sm p-1 text-low hover:text-normal focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand disabled:cursor-not-allowed disabled:opacity-50"
              aria-label={t('logs.previousMatch')}
              title={t('logs.previousMatchShortcut')}
            >
              <CaretUpIcon
                className="size-icon-sm"
                weight="bold"
                aria-hidden="true"
              />
            </button>
            <button
              onClick={onNextMatch}
              disabled={matchCount === 0}
              className="rounded-sm p-1 text-low hover:text-normal focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand disabled:cursor-not-allowed disabled:opacity-50"
              aria-label={t('logs.nextMatch')}
              title={t('logs.nextMatchShortcut')}
            >
              <CaretDownIcon
                className="size-icon-sm"
                weight="bold"
                aria-hidden="true"
              />
            </button>
          </div>
        </>
      )}
    </div>
  );

  const terminalItem = (
    <button
      type="button"
      onClick={expandTerminal}
      className={cn(
        'flex h-[26px] w-full items-center gap-half rounded-sm px-half text-left transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand'
      )}
      aria-label={t('processes.terminal')}
    >
      <TerminalIcon
        className="size-icon-sm flex-shrink-0 text-low"
        weight="regular"
        aria-hidden="true"
      />
      <span
        className={cn(
          'text-sm truncate flex-1',
          isTerminalExpanded ? 'text-high' : 'text-normal'
        )}
      >
        {t('processes.terminal')}
      </span>
    </button>
  );

  return (
    <div className="agentos-process-list flex flex-col flex-1 w-full bg-secondary">
      <div className="flex-1 overflow-y-auto pt-half px-base">
        {terminalItem}
        {(viewState === 'loading' || viewState === 'error') && (
          <ProcessListStatus viewState={viewState} onRetry={retry} />
        )}
        {viewState === 'processes-with-error' && (
          <ProcessListWarning onRetry={retry} />
        )}
        {sortedProcesses.map((process) => (
          <ProcessListItem
            key={process.id}
            runReason={process.run_reason}
            status={process.status}
            startedAt={process.started_at}
            selected={process.id === selectedProcessId}
            onClick={() => handleSelectProcess(process.id)}
          />
        ))}
      </div>
      {viewState === 'empty' && !isTerminalExpanded && (
        <div className="flex-1 flex items-center justify-center text-low">
          <p className="text-sm">{t('processes.noProcesses')}</p>
        </div>
      )}
      <div>{searchBar}</div>
    </div>
  );
}
