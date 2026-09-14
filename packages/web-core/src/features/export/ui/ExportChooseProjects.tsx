import { useState, useEffect } from 'react';
import {
  ArrowClockwiseIcon,
  CheckCircleIcon,
  CircleIcon,
  ImageIcon,
  WarningCircleIcon,
} from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import { deriveExportDataState } from './exportDataState';

export interface ExportOrganization {
  id: string;
  name: string;
}

export interface ExportProject {
  id: string;
  name: string;
}

interface ExportChooseProjectsProps {
  organizations: ExportOrganization[];
  orgsLoading: boolean;
  orgsError: boolean;
  projects: ExportProject[];
  projectsLoading: boolean;
  projectsError: boolean;
  selectedOrgId: string | null;
  onOrgChange: (orgId: string) => void;
  onRetryData: () => void;
  onContinue: (
    orgId: string,
    projectIds: string[],
    includeAttachments: boolean
  ) => void;
}

export function ExportChooseProjects({
  organizations,
  orgsLoading,
  orgsError,
  projects,
  projectsLoading,
  projectsError,
  selectedOrgId,
  onOrgChange,
  onRetryData,
  onContinue,
}: ExportChooseProjectsProps) {
  const { t } = useTranslation('common');
  const [selectedProjectIds, setSelectedProjectIds] = useState<Set<string>>(
    new Set()
  );
  const [includeAttachments, setIncludeAttachments] = useState(true);

  // Select all projects by default when they load
  useEffect(() => {
    if (projects.length > 0) {
      setSelectedProjectIds(new Set(projects.map((p) => p.id)));
    }
  }, [projects]);

  const handleToggleProject = (projectId: string) => {
    setSelectedProjectIds((prev) => {
      const next = new Set(prev);
      if (next.has(projectId)) {
        next.delete(projectId);
      } else {
        next.add(projectId);
      }
      return next;
    });
  };

  const handleSelectAll = () => {
    if (selectedProjectIds.size === projects.length) {
      setSelectedProjectIds(new Set());
    } else {
      setSelectedProjectIds(new Set(projects.map((p) => p.id)));
    }
  };

  const handleContinue = () => {
    if (selectedOrgId && selectedProjectIds.size > 0) {
      onContinue(
        selectedOrgId,
        Array.from(selectedProjectIds),
        includeAttachments
      );
    }
  };

  const isLoading = orgsLoading || projectsLoading;
  const organizationState = deriveExportDataState({
    isLoading: orgsLoading,
    hasError: orgsError,
    itemCount: organizations.length,
  });
  const projectState = deriveExportDataState({
    isLoading: projectsLoading,
    hasError: projectsError,
    itemCount: projects.length,
  });
  const dataUnavailable =
    organizationState === 'unavailable' || projectState === 'unavailable';
  const dataStale = organizationState === 'stale' || projectState === 'stale';

  return (
    <div className="p-double space-y-double">
      <div className="space-y-base">
        <h2 className="text-lg font-semibold text-high">{t('export.title')}</h2>
      </div>

      {(dataUnavailable || dataStale) && !isLoading && (
        <div
          className="flex items-start gap-half rounded-sm border border-warning/30 bg-warning/10 px-base py-half text-sm text-warning"
          role={dataStale ? 'status' : 'alert'}
        >
          <WarningCircleIcon
            className="mt-0.5 size-icon-sm shrink-0"
            weight="fill"
            aria-hidden="true"
          />
          <div className="min-w-0 flex-1">
            <p>
              {t(
                dataStale ? 'export.dataMayBeStale' : 'export.dataUnavailable'
              )}
            </p>
            <button
              type="button"
              onClick={onRetryData}
              className="mt-half inline-flex items-center gap-half font-medium text-warning underline-offset-2 hover:underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand"
            >
              <ArrowClockwiseIcon size={14} aria-hidden="true" />
              {t('export.retryData')}
            </button>
          </div>
        </div>
      )}

      {organizations.length > 1 && (
        <div className="space-y-half">
          <label
            htmlFor="export-organization"
            className="text-sm font-medium text-high"
          >
            {t('export.organization')}
          </label>
          <select
            id="export-organization"
            name="organizationId"
            value={selectedOrgId ?? ''}
            onChange={(e) => onOrgChange(e.target.value)}
            className="w-full rounded-sm border border-border bg-primary px-base py-half text-sm text-high focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand"
          >
            {organizations.map((org) => (
              <option key={org.id} value={org.id}>
                {org.name}
              </option>
            ))}
          </select>
        </div>
      )}

      {isLoading ? (
        <p className="text-sm text-low" role="status" aria-live="polite">
          {t('export.loadingProjects')}
        </p>
      ) : projects.length === 0 ? (
        <p className="text-sm text-low">{t('export.noProjects')}</p>
      ) : (
        <div className="space-y-half">
          <div className="flex items-center justify-between">
            <span className="text-sm text-normal">
              {t('export.selected', {
                selected: selectedProjectIds.size,
                total: projects.length,
              })}
            </span>
            <button
              type="button"
              onClick={handleSelectAll}
              className="rounded-sm px-half py-0.5 text-sm text-brand hover:bg-primary hover:text-brand/80 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand"
              aria-pressed={selectedProjectIds.size === projects.length}
            >
              {selectedProjectIds.size === projects.length
                ? t('export.deselectAll')
                : t('export.selectAll')}
            </button>
          </div>
          <div className="max-h-64 overflow-y-auto rounded-sm border border-border divide-y divide-border">
            {projects.map((project) => {
              const isSelected = selectedProjectIds.has(project.id);
              return (
                <button
                  key={project.id}
                  type="button"
                  onClick={() => handleToggleProject(project.id)}
                  className="flex min-h-10 w-full items-center gap-base px-base py-half text-left text-sm transition-colors hover:bg-primary focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-brand"
                  aria-pressed={isSelected}
                >
                  {isSelected ? (
                    <CheckCircleIcon
                      className="size-icon-sm text-brand shrink-0"
                      weight="fill"
                      aria-hidden="true"
                    />
                  ) : (
                    <CircleIcon
                      className="size-icon-sm text-low shrink-0"
                      aria-hidden="true"
                    />
                  )}
                  <span className={isSelected ? 'text-high' : 'text-normal'}>
                    {project.name}
                  </span>
                </button>
              );
            })}
          </div>
        </div>
      )}

      <label
        htmlFor="export-include-attachments"
        className="flex items-start gap-base cursor-pointer"
      >
        <input
          id="export-include-attachments"
          type="checkbox"
          checked={includeAttachments}
          onChange={(e) => setIncludeAttachments(e.target.checked)}
          className="mt-0.5 rounded border-border"
        />
        <div className="space-y-half">
          <div className="flex items-center gap-half">
            <ImageIcon
              className="size-icon-sm text-normal"
              aria-hidden="true"
            />
            <span className="text-sm font-medium text-high">
              {t('export.includeAttachments')}
            </span>
          </div>
          <p className="text-xs text-low">
            {t('export.includeAttachmentsDescription')}
          </p>
        </div>
      </label>

      <button
        type="button"
        onClick={handleContinue}
        disabled={selectedProjectIds.size === 0}
        className="w-full rounded-sm bg-brand px-base py-half text-sm font-medium text-white transition-colors hover:bg-brand/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
      >
        {t('export.start')}
      </button>
    </div>
  );
}
