import { cn } from '@/shared/lib/utils';

interface AgentOSWordmarkProps {
  className?: string;
  alt?: string;
}

/**
 * Shared product identity for surfaces that sit outside the global shell.
 * Keep the mark semantic and CSS-driven so onboarding, export and remote
 * entrypoints cannot drift into separate brand assets again.
 */
export function AgentOSWordmark({
  className,
  alt = 'AgentOS',
}: AgentOSWordmarkProps) {
  return (
    <div
      className={cn(
        'agentos-brand-wordmark inline-flex items-center gap-2',
        className
      )}
      role="img"
      aria-label={alt}
    >
      <span
        className="agentos-brand-wordmark__mark flex h-8 w-8 items-center justify-center rounded-md text-sm font-semibold"
        aria-hidden="true"
      >
        A
      </span>
      <span className="agentos-brand-wordmark__name text-sm font-semibold tracking-tight">
        AgentOS
      </span>
    </div>
  );
}
