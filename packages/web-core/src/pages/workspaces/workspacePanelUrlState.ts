import { zodValidator } from '@tanstack/zod-adapter';
import { z } from 'zod';

import type {
  MobileTab,
  RightMainPanelMode,
} from '@/shared/stores/useUiPreferencesStore';

export const WORKSPACE_URL_PANELS = [
  'workspaces',
  'chat',
  'changes',
  'logs',
  'preview',
  'git',
] as const;

export type WorkspaceUrlPanel = (typeof WORKSPACE_URL_PANELS)[number];

export type WorkspacePanelUrlChangeOptions = {
  replace: boolean;
};

const workspaceUrlPanelSchema = z.enum(WORKSPACE_URL_PANELS);

export const workspacePanelSearchSchema = z.object({
  panel: workspaceUrlPanelSchema.optional().catch(undefined),
});

export type WorkspacePanelSearch = z.infer<typeof workspacePanelSearchSchema>;

export const workspacePanelSearchValidator = zodValidator(
  workspacePanelSearchSchema
);

export function getRightMainPanelModeForUrlPanel(
  panel: WorkspaceUrlPanel
): RightMainPanelMode | null {
  switch (panel) {
    case 'changes':
    case 'logs':
    case 'preview':
      return panel;
    case 'workspaces':
    case 'chat':
    case 'git':
      return null;
  }
}

export function resolveWorkspacePanelUrlOverride(
  panel: WorkspaceUrlPanel | undefined
): {
  mobileTab: MobileTab;
  rightMainPanelMode: RightMainPanelMode | null;
} | null {
  if (panel === undefined) return null;

  return {
    mobileTab: panel,
    rightMainPanelMode: getRightMainPanelModeForUrlPanel(panel),
  };
}

export function getWorkspaceUrlPanelFromLayout({
  isMobile,
  mobileTab,
  rightMainPanelMode,
  currentUrlPanel,
}: {
  isMobile: boolean;
  mobileTab: MobileTab;
  rightMainPanelMode: RightMainPanelMode | null;
  currentUrlPanel: WorkspaceUrlPanel | undefined;
}): WorkspaceUrlPanel {
  if (isMobile) return mobileTab;

  if (
    currentUrlPanel !== undefined &&
    getRightMainPanelModeForUrlPanel(currentUrlPanel) === rightMainPanelMode
  ) {
    return currentUrlPanel;
  }

  return rightMainPanelMode ?? 'chat';
}
