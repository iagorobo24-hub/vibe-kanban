import { useCallback, useEffect, useMemo, useState } from 'react';
import { ExportLayout } from '@/features/export/ui/ExportLayout';
import { AgentOSWordmark } from '@/shared/components/AgentOSWordmark';
import type { ExportRequest } from '@/features/export/ui/ExportDownload';
import type {
  ExportOrganization,
  ExportProject,
} from '@/features/export/ui/ExportChooseProjects';
import { useAuth } from '@/shared/hooks/auth/useAuth';
import { useTranslation } from 'react-i18next';
import { useUserOrganizations } from '@/shared/hooks/useUserOrganizations';
import { useOrganizationProjects } from '@/shared/hooks/useOrganizationProjects';
import { makeRequest as makeRemoteRequest } from '@/shared/lib/remoteApi';
import { LoginRequiredPrompt } from '@/shared/dialogs/shared/LoginRequiredPrompt';

interface ExportPageProps {
  exportFn: (request: ExportRequest) => Promise<Response>;
  organizations: ExportOrganization[];
  orgsLoading: boolean;
  orgsError: boolean;
  projects: ExportProject[];
  projectsLoading: boolean;
  projectsError: boolean;
  selectedOrgId: string | null;
  onOrgChange: (orgId: string) => void;
  onRetryData: () => void;
}

export function ExportPage({
  exportFn,
  organizations,
  orgsLoading,
  orgsError,
  projects,
  projectsLoading,
  projectsError,
  selectedOrgId,
  onOrgChange,
  onRetryData,
}: ExportPageProps) {
  const { t } = useTranslation('common');

  return (
    <div className="agentos-theme agentos-export-page agentos-page-shell h-full overflow-auto bg-primary">
      <div className="agentos-export-page__content mx-auto flex min-h-full w-full max-w-3xl flex-col justify-center px-base py-double">
        <div className="agentos-export-page__card agentos-page-card rounded-sm border border-border bg-secondary p-double space-y-double">
          <header className="agentos-export-page__header space-y-double text-center">
            <div className="flex justify-center">
              <AgentOSWordmark />
            </div>
            <p className="text-sm text-low">{t('export.description')}</p>
          </header>
          <ExportLayout
            exportFn={exportFn}
            organizations={organizations}
            orgsLoading={orgsLoading}
            orgsError={orgsError}
            projects={projects}
            projectsLoading={projectsLoading}
            projectsError={projectsError}
            selectedOrgId={selectedOrgId}
            onOrgChange={onOrgChange}
            onRetryData={onRetryData}
          />
        </div>
      </div>
    </div>
  );
}

export function ExportPageContainer() {
  const { t } = useTranslation('common');
  const { isLoaded, isSignedIn } = useAuth();
  const {
    data: orgsData,
    isLoading: orgsLoading,
    error: orgsError,
    refetch: refetchOrganizations,
  } = useUserOrganizations();
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

  const {
    data: projectData = [],
    isLoading: projectsLoading,
    error: projectsError,
    retry: retryProjects,
  } = useOrganizationProjects(selectedOrgId);
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

  const onRetryData = useCallback(() => {
    void refetchOrganizations();
    retryProjects();
  }, [refetchOrganizations, retryProjects]);

  if (!isLoaded) {
    return (
      <div className="agentos-page-shell flex h-full w-full items-center justify-center bg-primary">
        <p className="text-sm text-low" role="status" aria-live="polite">
          {t('export.loading')}
        </p>
      </div>
    );
  }

  if (!isSignedIn) {
    return (
      <div className="agentos-page-shell flex h-full w-full items-center justify-center bg-primary p-base">
        <LoginRequiredPrompt
          className="max-w-md"
          title={t('export.signInTitle')}
          description={t('export.signInDescription')}
          actionLabel={t('export.signInAction')}
        />
      </div>
    );
  }

  return (
    <ExportPage
      exportFn={exportFn}
      organizations={organizations}
      orgsLoading={orgsLoading}
      orgsError={Boolean(orgsError)}
      projects={projects}
      projectsLoading={projectsLoading}
      projectsError={Boolean(projectsError)}
      selectedOrgId={selectedOrgId}
      onOrgChange={setSelectedOrgId}
      onRetryData={onRetryData}
    />
  );
}
