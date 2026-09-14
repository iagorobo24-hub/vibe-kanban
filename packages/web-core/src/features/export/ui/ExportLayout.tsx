import { useState } from 'react';
import {
  ExportChooseProjects,
  type ExportOrganization,
  type ExportProject,
} from './ExportChooseProjects';
import { ExportDownload, type ExportRequest } from './ExportDownload';

interface ExportLayoutProps {
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

interface ExportData {
  orgId: string;
  projectIds: string[];
  includeAttachments: boolean;
}

export function ExportLayout({
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
}: ExportLayoutProps) {
  const [exportData, setExportData] = useState<ExportData | null>(null);

  const handleChooseProjectsContinue = (
    orgId: string,
    projectIds: string[],
    includeAttachments: boolean
  ) => {
    setExportData({
      orgId,
      projectIds,
      includeAttachments,
    });
  };

  if (exportData) {
    return (
      <ExportDownload
        orgId={exportData.orgId}
        projectIds={exportData.projectIds}
        includeAttachments={exportData.includeAttachments}
        onExportMore={() => setExportData(null)}
        exportFn={exportFn}
      />
    );
  }

  return (
    <ExportChooseProjects
      organizations={organizations}
      orgsLoading={orgsLoading}
      orgsError={orgsError}
      projects={projects}
      projectsLoading={projectsLoading}
      projectsError={projectsError}
      selectedOrgId={selectedOrgId}
      onOrgChange={onOrgChange}
      onRetryData={onRetryData}
      onContinue={handleChooseProjectsContinue}
    />
  );
}
