import { useState, useEffect } from 'react';
import { CheckCircleIcon, CircleIcon, ImageIcon } from '@phosphor-icons/react';

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
  projects: ExportProject[];
  projectsLoading: boolean;
  selectedOrgId: string | null;
  onOrgChange: (orgId: string) => void;
  onContinue: (
    orgId: string,
    projectIds: string[],
    includeAttachments: boolean
  ) => void;
}

export function ExportChooseProjects({
  organizations,
  orgsLoading,
  projects,
  projectsLoading,
  selectedOrgId,
  onOrgChange,
  onContinue,
}: ExportChooseProjectsProps) {
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

  return (
    <div className="p-double space-y-double">
      <div className="space-y-base">
        <h2 className="text-lg font-semibold text-high">Export projects</h2>
      </div>

      {organizations.length > 1 && (
        <div className="space-y-half">
          <label
            htmlFor="export-organization"
            className="text-sm font-medium text-high"
          >
            Organization
          </label>
          <select
            id="export-organization"
            value={selectedOrgId ?? ''}
            onChange={(e) => onOrgChange(e.target.value)}
            className="w-full rounded-sm border border-border bg-primary px-base py-half text-sm text-high focus:outline-none focus:ring-1 focus:ring-brand"
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
        <p className="text-sm text-low">Loading projects...</p>
      ) : projects.length === 0 ? (
        <p className="text-sm text-low">No projects found.</p>
      ) : (
        <div className="space-y-half">
          <div className="flex items-center justify-between">
            <span className="text-sm text-normal">
              {selectedProjectIds.size} of {projects.length} selected
            </span>
            <button
              type="button"
              onClick={handleSelectAll}
              className="rounded-sm px-half py-0.5 text-sm text-brand hover:bg-primary hover:text-brand/80 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand"
              aria-pressed={selectedProjectIds.size === projects.length}
            >
              {selectedProjectIds.size === projects.length
                ? 'Deselect all'
                : 'Select all'}
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
              Include attachments
            </span>
          </div>
          <p className="text-xs text-low">Include files attached to issues.</p>
        </div>
      </label>

      <button
        type="button"
        onClick={handleContinue}
        disabled={selectedProjectIds.size === 0}
        className="w-full rounded-sm bg-brand px-base py-half text-sm font-medium text-white transition-colors hover:bg-brand/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
      >
        Export
      </button>
    </div>
  );
}
