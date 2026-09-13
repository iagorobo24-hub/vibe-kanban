import { WarningCircleIcon } from '@phosphor-icons/react';
import { cn } from '../lib/cn';

interface ChatErrorMessageProps {
  content: string;
  className?: string;
  expanded?: boolean;
  onToggle?: () => void;
}

export function ChatErrorMessage({
  content,
  className,
  expanded,
  onToggle,
}: ChatErrorMessageProps) {
  return (
    <button
      type="button"
      className={cn(
        'flex w-full items-start gap-base border-0 bg-transparent p-0 text-left text-sm text-error cursor-pointer',
        className
      )}
      onClick={onToggle}
      aria-expanded={expanded}
    >
      <WarningCircleIcon
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
