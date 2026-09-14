import { useCallback } from 'react';
import { createFileRoute } from '@tanstack/react-router';
import { Workspaces } from '@/pages/workspaces/Workspaces';
import {
  workspacePanelSearchValidator,
  type WorkspacePanelUrlChangeOptions,
  type WorkspaceUrlPanel,
} from '@/pages/workspaces/workspacePanelUrlState';

export const Route = createFileRoute(
  '/_app/hosts/$hostId/workspaces_/$workspaceId'
)({
  validateSearch: workspacePanelSearchValidator,
  component: WorkspaceRouteComponent,
});

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
    [navigate]
  );

  return <Workspaces urlPanel={panel} onUrlPanelChange={handlePanelChange} />;
}
