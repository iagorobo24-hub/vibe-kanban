import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

export interface AgentOSEmptyStateProps {
  title: string;
  description: string;
  action?: ReactNode;
  className?: string;
}

export function AgentOSEmptyState({
  title,
  description,
  action,
  className,
}: AgentOSEmptyStateProps) {
  return (
    <div className={cn("agentos-empty-state", className)}>
      <h3 className="agentos-empty-state__title">{title}</h3>
      <p className="agentos-empty-state__description">{description}</p>
      {action}
    </div>
  );
}
