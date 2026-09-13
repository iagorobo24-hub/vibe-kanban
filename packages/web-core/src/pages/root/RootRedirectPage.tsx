import { useEffect } from 'react';
import { useUserSystem } from '@/shared/hooks/useUserSystem';
import { getFirstProjectDestination } from '@/shared/lib/firstProjectDestination';
import { useOrganizationStore } from '@/shared/stores/useOrganizationStore';
import { useUiPreferencesStore } from '@/shared/stores/useUiPreferencesStore';
import { useAppNavigation } from '@/shared/hooks/useAppNavigation';

export function RootRedirectPage() {
  const { config, loading, loginStatus, remoteAuthDegraded } = useUserSystem();
  const setSelectedOrgId = useOrganizationStore((s) => s.setSelectedOrgId);
  const appNavigation = useAppNavigation();

  useEffect(() => {
    if (loading || !config) {
      return;
    }

    let isActive = true;
    void (async () => {
      if (!config.remote_onboarding_acknowledged) {
        appNavigation.goToOnboarding({ replace: true });
        return;
      }

      if (loginStatus?.status !== 'loggedin') {
        appNavigation.goToWorkspacesCreate({ replace: true });
        return;
      }

      // AgentOS is local-first: a degraded remote plane must never block the
      // root redirect while waiting for organizations/projects that cannot be
      // reached. The dashboard remains the operational entry point and lets
      // the user continue with local workspaces.
      if (remoteAuthDegraded) {
        appNavigation.goToWorkspaces({ replace: true });
        return;
      }

      // Read saved selections imperatively to avoid re-triggering this effect
      // when the scratch store initializes from the server
      const { selectedOrgId, selectedProjectId } =
        useUiPreferencesStore.getState();

      const destination = await getFirstProjectDestination(
        setSelectedOrgId,
        selectedOrgId,
        selectedProjectId
      );
      if (!isActive) {
        return;
      }

      if (destination?.kind === 'project') {
        appNavigation.goToProject(destination.projectId, { replace: true });
        return;
      }

      appNavigation.goToWorkspacesCreate({ replace: true });
    })();

    return () => {
      isActive = false;
    };
  }, [
    appNavigation,
    config,
    loading,
    loginStatus?.status,
    remoteAuthDegraded,
    setSelectedOrgId,
  ]);

  return (
    <div className="h-screen bg-primary flex items-center justify-center">
      <p className="text-low">Loading...</p>
    </div>
  );
}
