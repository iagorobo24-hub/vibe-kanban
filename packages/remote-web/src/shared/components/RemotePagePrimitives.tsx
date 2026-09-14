import type { ReactNode } from "react";
import { AgentOSWordmark } from "@vibe/web-core/agentos-wordmark";

interface RemotePageProps {
  children: ReactNode;
  className?: string;
  maxWidth?: "max-w-md" | "max-w-3xl";
}

export function RemotePage({
  children,
  className = "",
  maxWidth = "max-w-md",
}: RemotePageProps) {
  return (
    <div
      className={`agentos-theme agentos-remote-page agentos-page-shell h-screen overflow-auto bg-primary ${className}`}
    >
      <div
        className={`agentos-remote-page__content mx-auto flex min-h-full w-full ${maxWidth} flex-col justify-center px-base py-double`}
      >
        {children}
      </div>
    </div>
  );
}

export function RemoteCard({
  children,
  className = "",
}: {
  children: ReactNode;
  className?: string;
}) {
  return (
    <section
      className={`agentos-remote-card rounded-sm border border-border bg-secondary ${className}`}
    >
      {children}
    </section>
  );
}

export function RemoteStatusCard({
  title,
  variant = "default",
  children,
}: {
  title: string;
  variant?: "default" | "error";
  children: ReactNode;
}) {
  return (
    <RemotePage>
      <RemoteCard
        className={`agentos-remote-status-card space-y-base p-double ${
          variant === "error" ? "agentos-remote-status-card--error" : ""
        }`}
      >
        <AgentOSWordmark />
        <div>
          <h2
            className={`text-lg font-semibold ${
              variant === "error" ? "text-error" : "text-high"
            }`}
          >
            {title}
          </h2>
          {children}
        </div>
      </RemoteCard>
    </RemotePage>
  );
}
