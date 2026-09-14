import { forwardRef } from 'react';
import {
  ListMagnifyingGlassIcon,
  TerminalWindowIcon,
  FileTextIcon,
  GlobeIcon,
} from '@phosphor-icons/react';
import { cn } from '../lib/cn';
import { ToolStatusDot, type ToolStatusLike } from './ToolStatusDot';

interface ChatToolSummaryProps {
  summary: string;
  className?: string;
  expanded?: boolean;
  onToggle?: () => void;
  status?: ToolStatusLike;
  onViewContent?: () => void;
  toolName?: string;
  isTruncated?: boolean;
  /** The action type for determining the icon */
  actionType?: string;
}

export const ChatToolSummary = forwardRef<
  HTMLSpanElement,
  ChatToolSummaryProps
>(function ChatToolSummary(
  {
    summary,
    className,
    expanded,
    onToggle,
    status,
    onViewContent,
    toolName,
    isTruncated,
    actionType,
  },
  ref
) {
  // Can expand if text is truncated and onToggle is provided
  const canExpand = isTruncated && onToggle;
  const isClickable = Boolean(onViewContent || canExpand);

  const handleClick = () => {
    if (onViewContent) {
      onViewContent();
    } else if (canExpand) {
      onToggle();
    }
  };

  // Determine icon based on action type or tool name
  const getIcon = () => {
    if (toolName === 'Bash') return TerminalWindowIcon;
    switch (actionType) {
      case 'file_read':
        return FileTextIcon;
      case 'search':
        return ListMagnifyingGlassIcon;
      case 'web_fetch':
        return GlobeIcon;
      default:
        return ListMagnifyingGlassIcon;
    }
  };
  const Icon = getIcon();

  const content = (
    <>
      <span className="relative shrink-0 pt-0.5">
        <Icon aria-hidden="true" className="size-icon-base" />
        {status && (
          <ToolStatusDot
            status={status}
            className="absolute -bottom-0.5 -left-0.5"
          />
        )}
      </span>
      <span
        ref={ref}
        className={cn(
          !expanded && 'truncate',
          expanded && 'whitespace-pre-wrap break-all'
        )}
      >
        {summary}
      </span>
    </>
  );

  if (isClickable) {
    return (
      <button
        type="button"
        className={cn(
          'flex w-full items-center gap-base border-0 bg-transparent p-0 text-left text-sm text-low cursor-pointer',
          className
        )}
        onClick={handleClick}
        aria-expanded={onViewContent ? undefined : expanded}
      >
        {content}
      </button>
    );
  }

  return (
    <div className={cn('flex items-center gap-base text-sm text-low', className)}>
      {content}
    </div>
  );
});
