import { useMemo } from "react";
import { useParams } from "@tanstack/react-router";
import { SettingsDialog } from "@/shared/dialogs/settings/SettingsDialog";
import { useTranslation } from "@/i18n/useTranslation";

interface BlockedHostState {
  id: string;
  name: string | null;
  errorMessage?: string | null;
}

interface WorkspacesUnavailablePageProps {
  blockedHost?: BlockedHostState;
  isCheckingBlockedHost?: boolean;
}

export default function WorkspacesUnavailablePage({
  blockedHost,
  isCheckingBlockedHost = false,
}: WorkspacesUnavailablePageProps) {
  const { t } = useTranslation("common");
  const { hostId } = useParams({ strict: false });

  const selectedHostId = useMemo(
    () => blockedHost?.id ?? hostId ?? null,
    [blockedHost?.id, hostId],
  );

  const selectedHostName = useMemo(
    () => blockedHost?.name ?? selectedHostId,
    [blockedHost?.name, selectedHostId],
  );

  const isBlockedHostState = Boolean(blockedHost);

  const openRelaySettings = () => {
    void SettingsDialog.show({
      initialSection: "relay",
    });
  };

  return (
    <div className="agentos-remote-status-page mx-auto flex h-full w-full max-w-3xl items-center justify-center px-double py-double">
      <div className="agentos-remote-status-page__card agentos-remote-card w-full space-y-base rounded-sm border border-border bg-secondary p-double">
        <h1 className="text-xl font-semibold text-high">
          {t("remoteWorkspaceUnavailable.title")}
        </h1>

        {isCheckingBlockedHost ? (
          <p className="text-sm text-low">
            {t("remoteWorkspaceUnavailable.connectingTo")}{" "}
            <span className="font-medium text-high">
              {selectedHostName ?? t("remoteWorkspaceUnavailable.selectedHost")}
            </span>
            {t("remoteWorkspaceUnavailable.ellipsis")}
          </p>
        ) : isBlockedHostState ? (
          <div className="space-y-base">
            <div className="rounded-sm border border-warning/40 bg-warning/10 p-base">
              <p className="text-sm font-medium text-high">
                {t("remoteWorkspaceUnavailable.connectFailed", {
                  host:
                    selectedHostName ??
                    t("remoteWorkspaceUnavailable.selectedHost"),
                })}
              </p>
              <p className="mt-half text-sm text-low">
                {t("remoteWorkspaceUnavailable.hostUnreachable")}
              </p>
            </div>

            <ol className="list-inside list-decimal space-y-half text-sm text-low">
              <li>{t("remoteWorkspaceUnavailable.steps.openAgentOS")}</li>
              <li>{t("remoteWorkspaceUnavailable.steps.openRelaySettings")}</li>
            </ol>

            {blockedHost?.errorMessage && (
              <p className="break-all text-xs text-low">
                {t("remoteWorkspaceUnavailable.lastError", {
                  error: blockedHost.errorMessage,
                })}
              </p>
            )}
          </div>
        ) : (
          <p className="text-sm text-low">
            {t("remoteWorkspaceUnavailable.selectOnlineHost")}
          </p>
        )}

        <button
          type="button"
          onClick={openRelaySettings}
          className="agentos-button agentos-button--secondary agentos-button--sm"
        >
          {t("remoteWorkspaceUnavailable.openRelaySettings")}
        </button>

        {isBlockedHostState && (
          <p className="text-sm text-low">
            {t("remoteWorkspaceUnavailable.retryHint")}
          </p>
        )}
      </div>
    </div>
  );
}
