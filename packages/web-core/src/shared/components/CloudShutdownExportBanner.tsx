import { cn } from '@/shared/lib/utils';

interface CloudShutdownExportBannerProps {
  onClick: () => void;
}

export function CloudShutdownExportBanner({
  onClick,
}: CloudShutdownExportBannerProps) {
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
        Vibe Kanban Cloud is shutting down. Export your data within 30 days.
      </button>
      <a
        href="https://vibekanban.com/shutdown"
        target="_blank"
        rel="noreferrer"
        className="underline underline-offset-2"
      >
        Read more here.
      </a>
    </div>
  );
}
