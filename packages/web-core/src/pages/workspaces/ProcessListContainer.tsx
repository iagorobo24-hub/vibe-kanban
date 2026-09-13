import { useEffect, useMemo, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { useExecutionProcessesContext } from '@/shared/hooks/useExecutionProcessesContext';
import { useLogsPanel } from '@/shared/hooks/useLogsPanel';
import { ProcessListItem } from '@vibe/ui/components/ProcessListItem';
import { InputField } from '@vibe/ui/components/InputField';
import {
  CaretUpIcon,
  CaretDownIcon,
  TerminalIcon,
} from '@phosphor-icons/react';
import { cn } from '@/shared/lib/utils';

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
  const { t } = useTranslation('common');
  const { executionProcessesVisible } = useExecutionProcessesContext();

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
              aria-label="Previous match"
              title="Previous match (Shift+Enter)"
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
              aria-label="Next match"
              title="Next match (Enter)"
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
      {sortedProcesses.length === 0 && !isTerminalExpanded && (
        <div className="flex-1 flex items-center justify-center text-low">
          <p className="text-sm">{t('processes.noProcesses')}</p>
        </div>
      )}
      <div>{searchBar}</div>
    </div>
  );
}
