import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import {
  DataWithScrollModifier,
  ScrollModifier,
  VirtuosoMessageList,
  VirtuosoMessageListLicense,
  VirtuosoMessageListMethods,
  VirtuosoMessageListProps,
} from '@virtuoso.dev/message-list';
import {
  ArrowClockwiseIcon,
  WarningCircleIcon,
} from '@phosphor-icons/react/dist/ssr';
import RawLogText from '@/shared/components/RawLogText';
import { deriveLogStreamViewState } from './logStreamState';
import {
  INITIAL_TOP_ITEM,
  InitialDataScrollModifier,
  ScrollToBottomModifier as ScrollToLastItem,
} from '@/shared/lib/virtuoso-modifiers';
import type { PatchType } from 'shared/types';

export type LogEntry = Extract<
  PatchType,
  { type: 'STDOUT' } | { type: 'STDERR' }
>;

export interface VirtualizedProcessLogsProps {
  logs: LogEntry[];
  error: string | null;
  searchQuery: string;
  matchIndices: number[];
  currentMatchIndex: number;
  onRetry?: () => void;
}

type LogEntryWithKey = LogEntry & { key: string; originalIndex: number };

interface SearchContext {
  searchQuery: string;
  matchIndices: number[];
  currentMatchIndex: number;
}

const computeItemKey: VirtuosoMessageListProps<
  LogEntryWithKey,
  SearchContext
>['computeItemKey'] = ({ data }) => data.key;

const ItemContent: VirtuosoMessageListProps<
  LogEntryWithKey,
  SearchContext
>['ItemContent'] = ({ data, context }) => {
  const isMatch = context.matchIndices.includes(data.originalIndex);
  const isCurrentMatch =
    context.matchIndices[context.currentMatchIndex] === data.originalIndex;

  return (
    <RawLogText
      content={data.content}
      channel={data.type === 'STDERR' ? 'stderr' : 'stdout'}
      className="text-sm px-4 py-1"
      linkifyUrls
      searchQuery={isMatch ? context.searchQuery : undefined}
      isCurrentMatch={isCurrentMatch}
    />
  );
};

export function VirtualizedProcessLogs({
  logs,
  error,
  searchQuery,
  matchIndices,
  currentMatchIndex,
  onRetry,
}: VirtualizedProcessLogsProps) {
  const { t } = useTranslation('tasks');
  const viewState = deriveLogStreamViewState(logs.length, !!error);
  const displayError =
    error === 'Connection failed' ? t('processes.connectionFailed') : error;
  const [channelData, setChannelData] =
    useState<DataWithScrollModifier<LogEntryWithKey> | null>(null);
  const messageListRef = useRef<VirtuosoMessageListMethods<
    LogEntryWithKey,
    SearchContext
  > | null>(null);
  const hasInitializedRef = useRef(false);
  const prevCurrentMatchRef = useRef<number | undefined>(undefined);
  const isAtBottomRef = useRef(true);

  useEffect(() => {
    const timeoutId = setTimeout(() => {
      const logsWithKeys: LogEntryWithKey[] = logs.map((entry, index) => ({
        ...entry,
        key: `log-${index}`,
        originalIndex: index,
      }));

      // Use InitialDataScrollModifier (with purgeItemSizes) only on the
      // very first data load. For all subsequent updates, use ScrollToLastItem
      // which always jumps to the end — unlike auto-scroll-to-bottom which
      // only follows if the viewport is already at the bottom.
      let scrollModifier: ScrollModifier | null = null;
      if (!hasInitializedRef.current && logs.length > 0) {
        hasInitializedRef.current = true;
        scrollModifier = InitialDataScrollModifier;
      } else if (isAtBottomRef.current) {
        scrollModifier = ScrollToLastItem;
      }

      if (scrollModifier) {
        setChannelData({ data: logsWithKeys, scrollModifier });
      } else {
        setChannelData({ data: logsWithKeys });
      }
    }, 100);

    return () => clearTimeout(timeoutId);
  }, [logs]);

  // Scroll to current match when it changes
  useEffect(() => {
    if (
      matchIndices.length > 0 &&
      currentMatchIndex >= 0 &&
      currentMatchIndex !== prevCurrentMatchRef.current
    ) {
      const logIndex = matchIndices[currentMatchIndex];
      messageListRef.current?.scrollToItem({
        index: logIndex,
        align: 'center',
        behavior: 'smooth',
      });
      prevCurrentMatchRef.current = currentMatchIndex;
    }
  }, [currentMatchIndex, matchIndices]);

  if (viewState === 'empty') {
    return (
      <div className="h-full flex items-center justify-center">
        <p className="text-center text-muted-foreground text-sm">
          {t('processes.noLogsAvailable')}
        </p>
      </div>
    );
  }

  if (viewState === 'error') {
    return (
      <div className="h-full flex flex-col items-center justify-center gap-base px-base">
        <div className="flex items-center gap-half text-center text-destructive text-sm">
          <WarningCircleIcon
            className="size-icon-base shrink-0"
            aria-hidden="true"
          />
          <span>{displayError}</span>
        </div>
        {onRetry && (
          <button
            type="button"
            onClick={onRetry}
            className="inline-flex items-center gap-half rounded-sm border border-border bg-primary px-base py-half text-sm font-medium text-normal transition-colors hover:bg-tertiary focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand"
          >
            <ArrowClockwiseIcon
              className="size-icon-sm"
              weight="bold"
              aria-hidden="true"
            />
            {t('processes.retryLogs')}
          </button>
        )}
      </div>
    );
  }

  const context: SearchContext = {
    searchQuery,
    matchIndices,
    currentMatchIndex,
  };

  return (
    <div className="virtuoso-license-wrapper flex h-full min-h-0 flex-col overflow-hidden">
      {viewState === 'logs-with-error' && (
        <div
          className="flex shrink-0 items-center justify-between gap-base border-b border-warning/50 bg-warning/10 px-base py-half text-sm text-warning"
          role="alert"
        >
          <div className="flex min-w-0 items-center gap-half">
            <WarningCircleIcon
              className="size-icon-sm shrink-0"
              weight="fill"
              aria-hidden="true"
            />
            <span className="truncate">{displayError}</span>
          </div>
          {onRetry && (
            <button
              type="button"
              onClick={onRetry}
              className="inline-flex shrink-0 items-center gap-half rounded-sm px-half py-quarter font-medium text-warning transition-colors hover:bg-warning/15 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand"
            >
              <ArrowClockwiseIcon
                className="size-icon-sm"
                weight="bold"
                aria-hidden="true"
              />
              {t('processes.retryLogs')}
            </button>
          )}
        </div>
      )}
      <div className="min-h-0 flex-1">
        <VirtuosoMessageListLicense
          licenseKey={import.meta.env.VITE_PUBLIC_REACT_VIRTUOSO_LICENSE_KEY}
        >
          <VirtuosoMessageList<LogEntryWithKey, SearchContext>
            ref={messageListRef}
            className="h-full"
            data={channelData}
            context={context}
            initialLocation={INITIAL_TOP_ITEM}
            onScroll={(location) => {
              isAtBottomRef.current = location.isAtBottom;
            }}
            computeItemKey={computeItemKey}
            ItemContent={ItemContent}
          />
        </VirtuosoMessageListLicense>
      </div>
    </div>
  );
}
