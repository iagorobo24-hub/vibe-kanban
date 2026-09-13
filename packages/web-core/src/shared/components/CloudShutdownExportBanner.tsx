import { cn } from '@/shared/lib/utils';
import { useTranslation } from 'react-i18next';

interface CloudShutdownExportBannerProps {
  onClick: () => void;
}

export function CloudShutdownExportBanner({
  onClick,
}: CloudShutdownExportBannerProps) {
  const { t } = useTranslation('common');

  return (
    <div
      className={cn(
        'agentos-cloud-shutdown-banner flex w-full items-center justify-center gap-half border-b border-border bg-brand px-base py-half text-center',
        'text-sm font-medium text-on-brand hover:bg-brand-hover'
      )}
    >
      <button
        type="button"
        onClick={onClick}
        className="border-0 bg-transparent p-0 text-inherit underline-offset-2 hover:underline focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-on-brand"
      >
        {t('cloudShutdown.message')}
      </button>
      <a
        href="https://vibekanban.com/shutdown"
        target="_blank"
        rel="noreferrer"
        className="underline underline-offset-2 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-on-brand"
      >
        {t('cloudShutdown.moreInfo')}
      </a>
    </div>
  );
}
