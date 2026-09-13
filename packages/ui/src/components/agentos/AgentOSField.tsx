import type { ReactNode } from "react";
import { cn } from "../../lib/cn";

export interface AgentOSFieldProps {
  id: string;
  label: string;
  children: ReactNode;
  hint?: string;
  error?: string;
  required?: boolean;
  className?: string;
}

export function AgentOSField({
  id,
  label,
  children,
  hint,
  error,
  required = false,
  className,
}: AgentOSFieldProps) {
  const descriptionId = `${id}-description`;

  return (
    <div className={cn("agentos-field", className)}>
      <label className="agentos-field__label" htmlFor={id}>
        {label}
        {required && (
          <span className="agentos-field__required" aria-hidden="true">
            *
          </span>
        )}
      </label>
      {children}
      {(hint || error) && (
        <p
          id={descriptionId}
          className={error ? "agentos-field__error" : "agentos-field__hint"}
          role={error ? "alert" : undefined}
        >
          {error ?? hint}
        </p>
      )}
    </div>
  );
}
