import * as React from "react";
import { SpinnerIcon } from "@phosphor-icons/react";
import { cn } from "../../lib/cn";

export interface AgentOSButtonProps
  extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost" | "danger";
  size?: "sm" | "md" | "lg";
  loading?: boolean;
}

export const AgentOSButton = React.forwardRef<
  HTMLButtonElement,
  AgentOSButtonProps
>(
  (
    {
      className,
      children,
      variant = "secondary",
      size = "md",
      loading = false,
      disabled,
      type = "button",
      ...props
    },
    ref,
  ) => (
    <button
      ref={ref}
      type={type}
      className={cn(
        "agentos-button",
        `agentos-button--${variant}`,
        size !== "md" && `agentos-button--${size}`,
        className,
      )}
      disabled={disabled || loading}
      aria-busy={loading || undefined}
      {...props}
    >
      {loading && <SpinnerIcon aria-hidden="true" className="animate-spin" />}
      {children}
    </button>
  ),
);

AgentOSButton.displayName = "AgentOSButton";
