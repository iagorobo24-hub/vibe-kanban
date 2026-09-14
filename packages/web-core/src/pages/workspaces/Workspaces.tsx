import { WorkspacesLayout } from './WorkspacesLayout';
import type {
  WorkspacePanelUrlChangeOptions,
  WorkspaceUrlPanel,
} from './workspacePanelUrlState';

export type WorkspacesProps = {
  urlPanel?: WorkspaceUrlPanel;
  onUrlPanelChange?: (
    panel: WorkspaceUrlPanel,
    options: WorkspacePanelUrlChangeOptions
  ) => void;
};

export function Workspaces({
  urlPanel,
  onUrlPanelChange,
}: WorkspacesProps = {}) {
  return (
    <WorkspacesLayout urlPanel={urlPanel} onUrlPanelChange={onUrlPanelChange} />
  );
}
