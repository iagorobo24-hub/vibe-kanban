import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

export interface AgentOSFeedbackStateProps {
  kind: "loading" | "error";
  title: string;
  description: string;
  action?: ReactNode;
  className?: string;
}

export function AgentOSFeedbackState({
  kind,
  title,
  description,
  action,
  className,
}: AgentOSFeedbackStateProps) {
  return (
    <div
      className={cn("agentos-feedback-state", className)}
      role={kind === "error" ? "alert" : "status"}
      aria-live="polite"
    >
      <h3 className="agentos-feedback-state__title">{title}</h3>
      <p className="agentos-feedback-state__description">{description}</p>
      {action}
    </div>
  );
}
