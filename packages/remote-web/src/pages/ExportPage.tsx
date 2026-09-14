import { useState, useEffect, useCallback } from "react";
import { ExportPage as ExportPageUI } from "@/pages/export/ExportPage";
import {
  authenticatedFetch,
  listOrganizations,
  listOrganizationProjects,
} from "@remote/shared/lib/api";
import type { ExportRequest } from "@/features/export/ui/ExportDownload";

const API_BASE = import.meta.env.VITE_API_BASE_URL || "";

export default function ExportPage() {
  const [organizations, setOrganizations] = useState<
    { id: string; name: string }[]
  >([]);
  const [orgsLoading, setOrgsLoading] = useState(true);
  const [orgsError, setOrgsError] = useState(false);
  const [orgsRetryKey, setOrgsRetryKey] = useState(0);
  const [selectedOrgId, setSelectedOrgId] = useState<string | null>(null);
  const [projects, setProjects] = useState<{ id: string; name: string }[]>([]);
  const [projectsLoading, setProjectsLoading] = useState(false);
  const [projectsError, setProjectsError] = useState(false);
  const [projectsRetryKey, setProjectsRetryKey] = useState(0);

  // Fetch organizations on mount
  useEffect(() => {
    let cancelled = false;
    async function fetchOrgs() {
      setOrgsError(false);
      try {
        const data = await listOrganizations();
        if (!cancelled) {
          const orgs = data.organizations.map((o) => ({
            id: o.id,
            name: o.name,
          }));
          setOrganizations(orgs);
          if (orgs.length > 0) {
            setSelectedOrgId(orgs[0].id);
          }
        }
      } catch {
        if (!cancelled) setOrgsError(true);
      } finally {
        if (!cancelled) setOrgsLoading(false);
      }
    }
    void fetchOrgs();
    return () => {
      cancelled = true;
    };
  }, [orgsRetryKey]);

  // Fetch projects when org changes
  useEffect(() => {
    if (!selectedOrgId) {
      setProjects([]);
      setProjectsError(false);
      setProjectsLoading(false);
      return;
    }
    let cancelled = false;
    async function fetchProjects() {
      setProjectsLoading(true);
      setProjectsError(false);
      setProjects([]);
      try {
        const data = await listOrganizationProjects(selectedOrgId!);
        if (!cancelled) {
          setProjects(data.map((p) => ({ id: p.id, name: p.name })));
        }
      } catch {
        if (!cancelled) setProjectsError(true);
      } finally {
        if (!cancelled) setProjectsLoading(false);
      }
    }
    void fetchProjects();
    return () => {
      cancelled = true;
    };
  }, [projectsRetryKey, selectedOrgId]);

  const onRetryData = useCallback(() => {
    setOrgsRetryKey((key) => key + 1);
    setProjectsRetryKey((key) => key + 1);
  }, []);

  const exportFn = useCallback(async (request: ExportRequest) => {
    return authenticatedFetch(`${API_BASE}/v1/export`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(request),
    });
  }, []);

  return (
    <ExportPageUI
      exportFn={exportFn}
      organizations={organizations}
      orgsLoading={orgsLoading}
      orgsError={orgsError}
      projects={projects}
      projectsLoading={projectsLoading}
      projectsError={projectsError}
      selectedOrgId={selectedOrgId}
      onOrgChange={setSelectedOrgId}
      onRetryData={onRetryData}
    />
  );
}
