import * as React from "react";
import { cn } from "../../lib/cn";

export interface AgentOSIconButtonProps
  extends Omit<React.ButtonHTMLAttributes<HTMLButtonElement>, "aria-label"> {
  label: string;
  icon: React.ReactNode;
}

export const AgentOSIconButton = React.forwardRef<
  HTMLButtonElement,
  AgentOSIconButtonProps
>(({ className, label, icon, type = "button", ...props }, ref) => (
  <button
    ref={ref}
    type={type}
    className={cn("agentos-icon-button", className)}
    aria-label={label}
    title={props.title ?? label}
    {...props}
  >
    {icon}
  </button>
));

AgentOSIconButton.displayName = "AgentOSIconButton";
