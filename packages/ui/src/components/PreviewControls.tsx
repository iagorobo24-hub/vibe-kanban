import type { ReactNode } from 'react';
import { ArrowSquareOutIcon, SpinnerIcon } from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import { cn } from '../lib/cn';

export interface PreviewControlsProcessTab {
  id: string;
  label: string;
}

interface PreviewControlsProps {
  processTabs: PreviewControlsProcessTab[];
  activeProcessId: string | null;
  logsContent: ReactNode;
  onViewFullLogs: () => void;
  onTabChange: (processId: string) => void;
  isLoading: boolean;
  className?: string;
}

export function PreviewControls({
  processTabs,
  activeProcessId,
  logsContent,
  onViewFullLogs,
  onTabChange,
  isLoading,
  className,
}: PreviewControlsProps) {
  const { t } = useTranslation(['tasks', 'common']);

  return (
    <div
      className={cn(
        'w-full bg-secondary flex flex-col overflow-hidden',
        className
      )}
    >
      <div className="flex-1 flex flex-col min-h-0">
        <div className="flex items-center justify-between px-base py-half">
          <span className="text-xs font-medium text-low">
            {t('preview.logs.label')}
          </span>
          <button
            type="button"
            onClick={onViewFullLogs}
            className="flex items-center gap-half rounded-sm text-xs text-brand hover:text-brand-hover focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand"
          >
            <span>{t('preview.logs.viewFull')}</span>
            <ArrowSquareOutIcon className="size-icon-xs" aria-hidden="true" />
          </button>
        </div>

        {processTabs.length > 1 && (
          <div className="flex border-b border-border mx-base">
            {processTabs.map((process) => (
              <button
                key={process.id}
                type="button"
                aria-pressed={activeProcessId === process.id}
                className={cn(
                  'rounded-t-sm border-b-2 px-base py-half text-xs transition-colors focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand',
                  activeProcessId === process.id
                    ? 'border-brand text-normal'
                    : 'border-transparent text-low hover:text-normal'
                )}
                onClick={() => onTabChange(process.id)}
              >
                {process.label}
              </button>
            ))}
          </div>
        )}

        <div className="flex-1 min-h-0 overflow-hidden">
          {isLoading && processTabs.length === 0 ? (
            <div className="h-full flex items-center justify-center text-low">
              <SpinnerIcon
                className="size-icon-sm animate-spin"
                aria-hidden="true"
              />
            </div>
          ) : processTabs.length > 0 ? (
            logsContent
          ) : null}
        </div>
      </div>
    </div>
  );
}
