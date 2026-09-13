import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

export interface AgentOSStatusProps {
  label: string;
  tone?:
    | "neutral"
    | "running"
    | "success"
    | "attention"
    | "failed"
    | "danger"
    | "info";
  detail?: string;
  live?: boolean;
  icon?: ReactNode;
  className?: string;
}

export function AgentOSStatus({
  label,
  tone = "neutral",
  detail,
  live = false,
  icon,
  className,
}: AgentOSStatusProps) {
  return (
    <div
      className={cn("agentos-status", `agentos-status--${tone}`, className)}
      aria-live={live ? "polite" : undefined}
    >
      {icon ?? <span className="agentos-status__dot" aria-hidden="true" />}
      <span>
        <span className="agentos-status__label">{label}</span>
        {detail && <span className="agentos-status__detail">{detail}</span>}
      </span>
    </div>
  );
}
