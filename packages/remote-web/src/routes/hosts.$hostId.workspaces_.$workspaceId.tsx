import { useCallback } from "react";
import { createFileRoute } from "@tanstack/react-router";
import { requireAuthenticated } from "@remote/shared/lib/route-auth";
import { Workspaces } from "@/pages/workspaces/Workspaces";
import { RemoteWorkspacesPageShell } from "@remote/pages/RemoteWorkspacesPageShell";
import {
  workspacePanelSearchValidator,
  type WorkspacePanelUrlChangeOptions,
  type WorkspaceUrlPanel,
} from "@/pages/workspaces/workspacePanelUrlState";

export const Route = createFileRoute("/hosts/$hostId/workspaces_/$workspaceId")(
  {
    beforeLoad: async ({ location }) => {
      await requireAuthenticated(location);
    },
    validateSearch: workspacePanelSearchValidator,
    component: WorkspaceRouteComponent,
  },
);

function WorkspaceRouteComponent() {
  const { panel } = Route.useSearch();
  const navigate = Route.useNavigate();
  const handlePanelChange = useCallback(
    (nextPanel: WorkspaceUrlPanel, options: WorkspacePanelUrlChangeOptions) => {
      void navigate({
        search: { panel: nextPanel },
        replace: options.replace,
      });
    },
    [navigate],
  );

  return (
    <RemoteWorkspacesPageShell>
      <Workspaces urlPanel={panel} onUrlPanelChange={handlePanelChange} />
    </RemoteWorkspacesPageShell>
  );
}
