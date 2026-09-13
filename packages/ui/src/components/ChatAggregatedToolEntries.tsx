import { ListMagnifyingGlassIcon, CaretRightIcon } from '@phosphor-icons/react';
import { cn } from '../lib/cn';
import { ToolStatusDot, type ToolStatusLike } from './ToolStatusDot';

export interface AggregatedEntry {
  summary: string;
  status?: ToolStatusLike;
  expansionKey: string;
  content?: string;
  command?: string;
}

interface ChatAggregatedToolEntriesProps {
  entries: AggregatedEntry[];
  expanded: boolean;
  isHovered: boolean;
  onToggle: () => void;
  onHoverChange: (hovered: boolean) => void;
  /** Label to show before the count (e.g., "Read", "Search") */
  label: string;
  /** Unit label for counting (e.g., "file", "URL") - will be pluralized automatically */
  unit: string;
  icon?: React.ElementType;
  className?: string;
  onViewContent?: (index: number) => void;
}

export function ChatAggregatedToolEntries({
  entries,
  expanded,
  isHovered,
  onToggle,
  onHoverChange,
  label,
  unit,
  icon: Icon = ListMagnifyingGlassIcon,
  className,
  onViewContent,
}: ChatAggregatedToolEntriesProps) {
  if (entries.length === 0) return null;

  // If only one entry, don't aggregate
  if (entries.length === 1) {
    const entry = entries[0];
    const content = (
      <>
        <span className="relative shrink-0 pt-0.5">
          <Icon aria-hidden="true" className="size-icon-base" />
          {entry.status && (
            <ToolStatusDot
              status={entry.status}
              className="absolute -bottom-0.5 -left-0.5"
            />
          )}
        </span>
        <span className="truncate">{entry.summary}</span>
      </>
    );

    if (onViewContent) {
      return (
        <button
          type="button"
          className={cn(
            'flex w-full items-center gap-base border-0 bg-transparent p-0 text-left text-sm text-low cursor-pointer',
            className
          )}
          onClick={() => onViewContent(0)}
        >
          {content}
        </button>
      );
    }

    return (
      <div
        className={cn('flex items-center gap-base text-sm text-low', className)}
      >
        {content}
      </div>
    );
  }

  // Get the worst status among all entries for the aggregate indicator
  const aggregateStatus = entries.reduce<ToolStatusLike | undefined>(
    (worst, entry) => {
      if (!entry.status) return worst;
      if (!worst) return entry.status;

      // Priority: failed > denied > timed_out > pending_approval > created > success
      const statusPriority: Record<string, number> = {
        failed: 6,
        denied: 5,
        timed_out: 4,
        pending_approval: 3,
        created: 2,
        success: 1,
      };

      const worstPriority = statusPriority[worst.status] || 0;
      const currentPriority = statusPriority[entry.status.status] || 0;

      return currentPriority > worstPriority ? entry.status : worst;
    },
    undefined
  );

  return (
    <div className={cn('flex flex-col', className)}>
      {/* Header row - clickable to expand/collapse */}
      <button
        type="button"
        className="flex w-full items-center gap-base border-0 bg-transparent p-0 text-left text-sm text-low cursor-pointer group"
        onClick={onToggle}
        onMouseEnter={() => onHoverChange(true)}
        onMouseLeave={() => onHoverChange(false)}
        aria-expanded={expanded}
        data-scroll-anchor-target=""
      >
        <span className="relative shrink-0 pt-0.5">
          {isHovered ? (
            <CaretRightIcon
              aria-hidden="true"
              className={cn(
                'size-icon-base transition-transform duration-150',
                expanded && 'rotate-90'
              )}
            />
          ) : (
            <Icon aria-hidden="true" className="size-icon-base" />
          )}
          {aggregateStatus && (
            <ToolStatusDot
              status={aggregateStatus}
              className="absolute -bottom-0.5 -left-0.5"
            />
          )}
        </span>
        <span className="truncate">
          {label} · {entries.length} {entries.length === 1 ? unit : `${unit}s`}
        </span>
      </button>

      {/* Expanded content */}
      {expanded && (
        <div className="ml-6 pt-1 flex flex-col gap-0.5">
          {entries.map((entry, index) => {
            const content = (
              <>
                <span className="relative shrink-0 pt-0.5">
                  <Icon aria-hidden="true" className="size-icon-base" />
                  {entry.status && (
                    <ToolStatusDot
                      status={entry.status}
                      className="absolute -bottom-0.5 -left-0.5"
                    />
                  )}
                </span>
                <span className="truncate">{entry.summary}</span>
              </>
            );
            const itemClassName = cn(
              'flex items-center gap-base text-sm text-low pl-base',
              onViewContent && 'cursor-pointer hover:text-normal'
            );

            return onViewContent ? (
              <button
                key={entry.expansionKey}
                type="button"
                className={cn(
                  itemClassName,
                  'w-full border-0 bg-transparent p-0 text-left'
                )}
                onClick={() => onViewContent(index)}
              >
                {content}
              </button>
            ) : (
              <div key={entry.expansionKey} className={itemClassName}>
                {content}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
