import { cn } from "../lib/cn";

interface AgentOSMarkProps {
  className?: string;
}

/**
 * Single source for the temporary AgentOS mark.
 *
 * The `A` is intentionally still a placeholder until the human-approved
 * identity direction is integrated. Keeping it here prevents the rail and
 * wordmark from drifting while that decision is pending.
 */
export function AgentOSMark({ className }: AgentOSMarkProps) {
  return (
    <span className={cn("agentos-brand-mark", className)} aria-hidden="true">
      A
    </span>
  );
}
