import { InfoIcon } from '@phosphor-icons/react';
import { cn } from '../lib/cn';

interface ChatSystemMessageProps {
  content: string;
  className?: string;
  expanded?: boolean;
  onToggle?: () => void;
}

export function ChatSystemMessage({
  content,
  className,
  expanded,
  onToggle,
}: ChatSystemMessageProps) {
  return (
    <button
      type="button"
      className={cn(
        'flex w-full items-start gap-base border-0 bg-transparent p-0 text-left text-sm text-low cursor-pointer',
        className
      )}
      onClick={onToggle}
      aria-expanded={expanded}
    >
      <InfoIcon
        aria-hidden="true"
        className="shrink-0 size-icon-base pt-0.5"
      />
      <span
        className={cn(
          !expanded && 'truncate',
          expanded && 'whitespace-pre-wrap break-all'
        )}
      >
        {content}
      </span>
    </button>
  );
}
