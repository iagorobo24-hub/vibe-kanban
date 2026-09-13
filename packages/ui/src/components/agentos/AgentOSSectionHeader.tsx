import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

export interface AgentOSSectionHeaderProps {
  title: string;
  eyebrow?: string;
  description?: string;
  action?: ReactNode;
  level?: "h2" | "h3";
  className?: string;
}

export function AgentOSSectionHeader({
  title,
  eyebrow,
  description,
  action,
  level = "h2",
  className,
}: AgentOSSectionHeaderProps) {
  const Heading = level;

  return (
    <div className={cn("agentos-section-header", className)}>
      <div>
        {eyebrow && (
          <p className="agentos-section-header__eyebrow">{eyebrow}</p>
        )}
        <Heading className="agentos-section-header__title">{title}</Heading>
        {description && (
          <p className="agentos-section-header__description">{description}</p>
        )}
      </div>
      {action}
    </div>
  );
}
