import { useCallback, useEffect, useMemo, useState } from 'react';
import { ExportLayout } from '@/features/export/ui/ExportLayout';
import { AgentOSWordmark } from '@/shared/components/AgentOSWordmark';
import type { ExportRequest } from '@/features/export/ui/ExportDownload';
import type {
  ExportOrganization,
  ExportProject,
} from '@/features/export/ui/ExportChooseProjects';
import { useAuth } from '@/shared/hooks/auth/useAuth';
import { useUserOrganizations } from '@/shared/hooks/useUserOrganizations';
import { useOrganizationProjects } from '@/shared/hooks/useOrganizationProjects';
import { makeRequest as makeRemoteRequest } from '@/shared/lib/remoteApi';
import { LoginRequiredPrompt } from '@/shared/dialogs/shared/LoginRequiredPrompt';

interface ExportPageProps {
  exportFn: (request: ExportRequest) => Promise<Response>;
  organizations: ExportOrganization[];
  orgsLoading: boolean;
  projects: ExportProject[];
  projectsLoading: boolean;
  selectedOrgId: string | null;
  onOrgChange: (orgId: string) => void;
}

export function ExportPage({
  exportFn,
  organizations,
  orgsLoading,
  projects,
  projectsLoading,
  selectedOrgId,
  onOrgChange,
}: ExportPageProps) {
  return (
    <div className="agentos-theme agentos-export-page agentos-page-shell h-full overflow-auto bg-primary">
      <div className="agentos-export-page__content mx-auto flex min-h-full w-full max-w-3xl flex-col justify-center px-base py-double">
        <div className="agentos-export-page__card agentos-page-card rounded-sm border border-border bg-secondary p-double space-y-double">
          <header className="agentos-export-page__header space-y-double text-center">
            <div className="flex justify-center">
              <AgentOSWordmark />
            </div>
            <p className="text-sm text-low">
              Download your project and issue data to CSV files. Optionally
              downloads your file attachments too.
            </p>
          </header>
          <ExportLayout
            exportFn={exportFn}
            organizations={organizations}
            orgsLoading={orgsLoading}
            projects={projects}
            projectsLoading={projectsLoading}
            selectedOrgId={selectedOrgId}
            onOrgChange={onOrgChange}
          />
        </div>
      </div>
    </div>
  );
}

export function ExportPageContainer() {
  const { isLoaded, isSignedIn } = useAuth();
  const { data: orgsData, isLoading: orgsLoading } = useUserOrganizations();
  const organizations = useMemo<ExportOrganization[]>(
    () =>
      (orgsData?.organizations ?? []).map((organization) => ({
        id: organization.id,
        name: organization.name,
      })),
    [orgsData?.organizations]
  );
  const [selectedOrgId, setSelectedOrgId] = useState<string | null>(null);

  useEffect(() => {
    if (organizations.length === 0) {
      return;
    }

    const hasSelectedOrg = selectedOrgId
      ? organizations.some((organization) => organization.id === selectedOrgId)
      : false;

    if (!hasSelectedOrg) {
      setSelectedOrgId(organizations[0].id);
    }
  }, [organizations, selectedOrgId]);

  const { data: projectData = [], isLoading: projectsLoading } =
    useOrganizationProjects(selectedOrgId);
  const projects = useMemo<ExportProject[]>(
    () =>
      projectData.map((project) => ({
        id: project.id,
        name: project.name,
      })),
    [projectData]
  );

  const exportFn = useCallback(async (request: ExportRequest) => {
    return makeRemoteRequest('/v1/export', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(request),
    });
  }, []);

  if (!isLoaded) {
    return (
      <div className="agentos-page-shell flex h-full w-full items-center justify-center bg-primary">
        <p className="text-sm text-low">Loading...</p>
      </div>
    );
  }

  if (!isSignedIn) {
    return (
      <div className="agentos-page-shell flex h-full w-full items-center justify-center bg-primary p-base">
        <LoginRequiredPrompt
          className="max-w-md"
          title="Sign in to export your cloud data"
          description="Sign in to choose the organizations and projects available to your account."
          actionLabel="Sign in"
        />
      </div>
    );
  }

  return (
    <ExportPage
      exportFn={exportFn}
      organizations={organizations}
      orgsLoading={orgsLoading}
      projects={projects}
      projectsLoading={projectsLoading}
      selectedOrgId={selectedOrgId}
      onOrgChange={setSelectedOrgId}
    />
  );
}
