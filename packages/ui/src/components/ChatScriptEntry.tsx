import { useTranslation } from 'react-i18next';
import { TerminalIcon, WrenchIcon } from '@phosphor-icons/react';
import { cn } from '../lib/cn';
import { ToolStatusDot, type ToolStatusLike } from './ToolStatusDot';

interface ChatScriptEntryProps {
  title: string;
  command?: string;
  processId: string;
  exitCode?: number | null;
  className?: string;
  status: ToolStatusLike;
  onViewProcess: (processId: string) => void;
  onFix?: () => void;
}

export function ChatScriptEntry({
  title,
  command,
  processId,
  exitCode,
  className,
  status,
  onViewProcess,
  onFix,
}: ChatScriptEntryProps) {
  const { t } = useTranslation('tasks');
  const isRunning = status.status === 'created';
  const isSuccess = status.status === 'success';
  const isFailed = status.status === 'failed';

  const handleFixClick = () => {
    onFix?.();
  };

  const handleClick = () => {
    onViewProcess(processId);
  };

  const getSubtitle = () => {
    if (isRunning) {
      return t('conversation.script.running');
    }
    if (isFailed && exitCode !== null && exitCode !== undefined) {
      return t('conversation.script.exitCode', { code: exitCode });
    }
    if (isSuccess) {
      return t('conversation.script.completedSuccessfully');
    }
    return t('conversation.script.clickToViewLogs');
  };

  return (
    <div
      className={cn(
        'flex items-start gap-base text-sm hover:bg-secondary/50 rounded-md -mx-half px-half py-half transition-colors',
        className
      )}
    >
      <button
        type="button"
        className="flex min-w-0 flex-1 items-start gap-base border-0 bg-transparent p-0 text-left focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand"
        onClick={handleClick}
        aria-label={`${title}: ${getSubtitle()}`}
      >
        <span className="relative shrink-0 pt-0.5">
          <TerminalIcon className="size-icon-base text-low" aria-hidden="true" />
          <ToolStatusDot
            status={status}
            className="absolute -bottom-0.5 -left-0.5"
          />
        </span>
        <span className="flex flex-col min-w-0 flex-1">
          <span className="text-normal font-medium">{title}</span>
          {command && (
            <code className="text-low text-xs font-mono truncate block">
              {command}
            </code>
          )}
          <span className="text-low text-xs">{getSubtitle()}</span>
        </span>
      </button>
      {isFailed && onFix && (
        <button
          type="button"
          onClick={handleFixClick}
          className="shrink-0 flex items-center gap-1 px-2 py-1 text-xs text-brand hover:text-brand-hover hover:bg-secondary rounded transition-colors"
          title={t('scriptFixer.fixScript')}
        >
          <WrenchIcon className="size-icon-xs" aria-hidden="true" />
          <span>{t('scriptFixer.fixScript')}</span>
        </button>
      )}
    </div>
  );
}
