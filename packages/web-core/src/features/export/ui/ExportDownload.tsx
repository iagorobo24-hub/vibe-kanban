import { useState, useEffect, useCallback, useRef } from 'react';
import {
  CheckCircleIcon,
  SpinnerIcon,
  WarningIcon,
  DownloadSimpleIcon,
} from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';

export interface ExportRequest {
  organization_id: string;
  project_ids: string[];
  include_attachments: boolean;
}

interface ExportDownloadProps {
  orgId: string;
  projectIds: string[];
  includeAttachments: boolean;
  onExportMore: () => void;
  exportFn: (request: ExportRequest) => Promise<Response>;
}

export function ExportDownload({
  orgId,
  projectIds,
  includeAttachments,
  onExportMore,
  exportFn,
}: ExportDownloadProps) {
  const { t } = useTranslation('common');
  const [isExporting, setIsExporting] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [downloadUrl, setDownloadUrl] = useState<string | null>(null);
  const [filename, setFilename] = useState('agentos-export.zip');
  const hasStartedRef = useRef(false);

  const startExport = useCallback(async () => {
    setIsExporting(true);
    setError(null);
    setDownloadUrl(null);

    try {
      const response = await exportFn({
        organization_id: orgId,
        project_ids: projectIds,
        include_attachments: includeAttachments,
      });

      if (!response.ok) {
        throw new Error(
          t('export.errors.requestFailed', { status: response.status })
        );
      }

      let downloadFilename = 'agentos-export.zip';
      const disposition = response.headers.get('content-disposition');
      if (disposition) {
        const match = disposition.match(/filename="?([^"]+)"?/);
        if (match) {
          downloadFilename = match[1];
        }
      }
      setFilename(downloadFilename);

      const blob = await response.blob();
      const url = URL.createObjectURL(blob);
      setDownloadUrl(url);

      const a = document.createElement('a');
      a.href = url;
      a.download = downloadFilename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
    } catch (err) {
      setError(err instanceof Error ? err.message : t('export.errors.generic'));
    } finally {
      setIsExporting(false);
    }
  }, [orgId, projectIds, includeAttachments, exportFn]);

  useEffect(() => {
    if (hasStartedRef.current) {
      return;
    }
    hasStartedRef.current = true;
    void startExport();
  }, [startExport]);

  useEffect(() => {
    return () => {
      if (downloadUrl) {
        URL.revokeObjectURL(downloadUrl);
      }
    };
  }, [downloadUrl]);

  const handleManualDownload = () => {
    if (downloadUrl) {
      const a = document.createElement('a');
      a.href = downloadUrl;
      a.download = filename;
      document.body.appendChild(a);
      a.click();
      document.body.removeChild(a);
    }
  };

  return (
    <div className="p-double space-y-double">
      {isExporting && (
        <div className="flex flex-col items-center gap-base py-double">
          <SpinnerIcon
            className="size-icon-lg text-brand animate-spin"
            weight="bold"
            aria-hidden="true"
          />
          <div className="text-center space-y-half">
            <p className="text-sm font-medium text-high">
              {t('export.generating')}
            </p>
            <p className="text-xs text-low">
              {t('export.wait', {
                attachments: includeAttachments
                  ? t('export.withAttachments')
                  : '',
              })}
            </p>
          </div>
        </div>
      )}

      {error && (
        <div className="space-y-base" role="alert">
          <div className="flex items-center gap-base text-danger">
            <WarningIcon
              className="size-icon-sm"
              weight="fill"
              aria-hidden="true"
            />
            <p className="text-sm font-medium">{t('export.errors.failed')}</p>
          </div>
          <p className="text-sm text-normal">{error}</p>
          <button
            type="button"
            onClick={() => {
              hasStartedRef.current = false;
              void startExport();
            }}
            className="w-full rounded-sm border border-border bg-secondary px-base py-half text-sm font-medium text-normal transition-colors hover:bg-primary focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand"
          >
            {t('export.retry')}
          </button>
        </div>
      )}

      {!isExporting && !error && downloadUrl && (
        <div className="space-y-double">
          <div className="flex flex-col items-center gap-base py-base">
            <CheckCircleIcon
              className="size-icon-lg text-success"
              weight="fill"
            />
            <div className="text-center space-y-half">
              <p className="text-sm font-medium text-high">
                {t('export.complete')}
              </p>
              <p className="text-xs text-low">
                {t('export.automaticDownload')}
              </p>
            </div>
          </div>

          <div className="space-y-base">
            <button
              type="button"
              onClick={handleManualDownload}
              className="flex w-full items-center justify-center gap-half rounded-sm bg-brand px-base py-half text-sm font-medium text-white transition-colors hover:bg-brand/90 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand focus-visible:ring-offset-2"
            >
              <DownloadSimpleIcon className="size-icon-sm" aria-hidden="true" />
              {t('export.download', { filename })}
            </button>

            <button
              type="button"
              onClick={onExportMore}
              className="w-full rounded-sm border border-border bg-secondary px-base py-half text-sm font-medium text-normal transition-colors hover:bg-primary focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand"
            >
              {t('export.more')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
