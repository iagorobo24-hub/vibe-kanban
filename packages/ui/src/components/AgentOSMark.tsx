import { cn } from "../lib/cn";
import orbitMarkUrl from "../assets/agentos-orbit-mark.svg";

interface AgentOSMarkProps {
  className?: string;
}

export function AgentOSMark({ className }: AgentOSMarkProps) {
  return (
    <span className={cn("agentos-brand-mark inline-flex shrink-0", className)} aria-hidden="true">
      <img src={orbitMarkUrl} alt="" className="h-full w-full object-contain" />
    </span>
  );
}
