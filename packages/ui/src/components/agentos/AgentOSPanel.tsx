import type { HTMLAttributes } from "react";
import { cn } from "../../lib/cn";

export interface AgentOSPanelProps extends HTMLAttributes<HTMLDivElement> {
  tone?: "default" | "raised" | "quiet";
}

export function AgentOSPanel({
  className,
  tone = "default",
  ...props
}: AgentOSPanelProps) {
  return (
    <div
      className={cn("agentos-panel", `agentos-panel--${tone}`, className)}
      {...props}
    />
  );
}
