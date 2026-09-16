import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { create, useModal } from '@ebay/nice-modal-react';
import { useQueryClient } from '@tanstack/react-query';
import { Loader2, Sparkles, CheckCircle, AlertCircle, HelpCircle } from 'lucide-react';
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@vibe/ui/components/KeyboardDialog';
import { Button } from '@vibe/ui/components/Button';
import { Label } from '@vibe/ui/components/Label';
import { AutoExpandingTextarea } from '@vibe/ui/components/AutoExpandingTextarea';
import { defineModal } from '@/shared/lib/modals';
import { repoApi, type DetectedScripts } from '@/shared/lib/api';

export interface AutoDetectScriptsDialogProps {
  repoId: string;
  repoName?: string;
  onApplied?: () => void;
}

export type AutoDetectScriptsDialogResult = {
  action: 'applied' | 'canceled';
};

const AutoDetectScriptsDialogImpl = create<AutoDetectScriptsDialogProps>(
  ({ repoId, repoName, onApplied }) => {
    const modal = useModal();
    const { t } = useTranslation(['tasks', 'common']);
    const queryClient = useQueryClient();

    const [isLoading, setIsLoading] = useState(true);
    const [isApplying, setIsApplying] = useState(false);
    const [error, setError] = useState<string | null>(null);

    const [detected, setDetected] = useState<DetectedScripts | null>(null);
    const [setupScript, setSetupScript] = useState('');
    const [cleanupScript, setCleanupScript] = useState('');
    const [devServerScript, setDevServerScript] = useState('');
    const [copyFiles, setCopyFiles] = useState('');

    useEffect(() => {
      let isMounted = true;
      setIsLoading(true);
      setError(null);

      repoApi
        .detectScripts(repoId)
        .then((res) => {
          if (!isMounted) return;
          setDetected(res);
          setSetupScript(res.setup_script || '');
          setCleanupScript(res.cleanup_script || '');
          setDevServerScript(res.dev_server_script || '');
          setCopyFiles(res.copy_files || '');
        })
        .catch((err) => {
          if (!isMounted) return;
          setError(
            err instanceof Error
              ? err.message
              : 'Error al detectar scripts del repositorio'
          );
        })
        .finally(() => {
          if (isMounted) setIsLoading(false);
        });

      return () => {
        isMounted = false;
      };
    }, [repoId]);

    const handleApply = useCallback(async () => {
      if (!detected) return;
      setIsApplying(true);
      setError(null);

      try {
        await repoApi.applyDetectedScripts(repoId, {
          ...detected,
          setup_script: setupScript.trim() || null,
          cleanup_script: cleanupScript.trim() || null,
          dev_server_script: devServerScript.trim() || null,
          copy_files: copyFiles.trim() || null,
        });

        queryClient.invalidateQueries({ queryKey: ['repos'] });
        queryClient.invalidateQueries({ queryKey: ['repo', repoId] });
        onApplied?.();
        modal.resolve({ action: 'applied' });
        modal.hide();
      } catch (err) {
        setError(
          err instanceof Error
            ? err.message
            : 'Error al aplicar scripts detectados'
        );
      } finally {
        setIsApplying(false);
      }
    }, [
      detected,
      repoId,
      setupScript,
      cleanupScript,
      devServerScript,
      copyFiles,
      queryClient,
      onApplied,
      modal,
    ]);

    const handleCancel = useCallback(() => {
      modal.resolve({ action: 'canceled' });
      modal.hide();
    }, [modal]);

    const renderConfidenceBadge = (confidence?: string) => {
      switch (confidence) {
        case 'high':
          return (
            <span className="inline-flex items-center gap-1 rounded-full bg-success/15 px-2 py-0.5 text-xs font-medium text-success">
              <CheckCircle className="h-3 w-3" />
              Confianza Alta
            </span>
          );
        case 'medium':
          return (
            <span className="inline-flex items-center gap-1 rounded-full bg-amber-500/15 px-2 py-0.5 text-xs font-medium text-amber-600 dark:text-amber-400">
              <AlertCircle className="h-3 w-3" />
              Confianza Media
            </span>
          );
        default:
          return (
            <span className="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-xs font-medium text-low">
              <HelpCircle className="h-3 w-3" />
              Stack No Identificado
            </span>
          );
      }
    };

    return (
      <Dialog open={modal.visible} onOpenChange={(open) => !open && handleCancel()}>
        <DialogContent className="max-w-xl">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-base font-semibold">
              <Sparkles className="h-5 w-5 text-brand" />
              Auto-Detección de Scripts {repoName ? `— ${repoName}` : ''}
            </DialogTitle>
          </DialogHeader>

          {isLoading ? (
            <div className="flex flex-col items-center justify-center py-8 gap-2 text-sm text-low">
              <Loader2 className="h-6 w-6 animate-spin text-brand" />
              <span>Analizando estructura y dependencias del repositorio...</span>
            </div>
          ) : error ? (
            <div className="rounded-sm border border-error/30 bg-error/10 p-3 text-sm text-error">
              {error}
            </div>
          ) : (
            <div className="flex flex-col gap-4 py-2">
              {/* Stack summary header */}
              <div className="flex items-center justify-between rounded-sm border border-border/60 bg-muted/40 p-3">
                <div>
                  <div className="text-xs text-low">Stack detectado</div>
                  <div className="text-sm font-semibold text-normal">
                    {detected?.stack_description || 'No identificado'}
                  </div>
                </div>
                {renderConfidenceBadge(detected?.confidence)}
              </div>

              {/* Detection notes */}
              {detected && detected.detection_notes.length > 0 && (
                <div className="rounded-sm bg-brand/5 border border-brand/15 px-3 py-2 text-xs text-normal">
                  <div className="font-semibold text-brand mb-1">Evidencias detectadas:</div>
                  <ul className="list-disc pl-4 space-y-0.5 text-low">
                    {detected.detection_notes.map((note, idx) => (
                      <li key={idx}>{note}</li>
                    ))}
                  </ul>
                </div>
              )}

              {/* Setup Script */}
              <div className="flex flex-col gap-1.5">
                <Label className="text-xs font-medium">
                  Script de Arranque (Setup Script)
                  <span className="ml-1 text-low font-normal">
                    — se ejecuta antes de que el agente comience
                  </span>
                </Label>
                <AutoExpandingTextarea
                  value={setupScript}
                  onChange={(e) => setSetupScript(e.target.value)}
                  placeholder="e.g. npm install o cargo build"
                  className="font-mono text-xs"
                  rows={2}
                />
              </div>

              {/* Cleanup Script */}
              <div className="flex flex-col gap-1.5">
                <Label className="text-xs font-medium">
                  Script de Finalización (Cleanup Script)
                  <span className="ml-1 text-low font-normal">
                    — se ejecuta tras el trabajo del agente si modificó código
                  </span>
                </Label>
                <AutoExpandingTextarea
                  value={cleanupScript}
                  onChange={(e) => setCleanupScript(e.target.value)}
                  placeholder="e.g. npm run lint --fix o cargo fmt"
                  className="font-mono text-xs"
                  rows={2}
                />
              </div>

              {/* Dev server / Copy files in collapsed/compact row */}
              <div className="grid grid-cols-2 gap-3">
                <div className="flex flex-col gap-1">
                  <Label className="text-xs font-medium">Dev Server (Opcional)</Label>
                  <AutoExpandingTextarea
                    value={devServerScript}
                    onChange={(e) => setDevServerScript(e.target.value)}
                    placeholder="e.g. npm run dev"
                    className="font-mono text-xs"
                    rows={1}
                  />
                </div>
                <div className="flex flex-col gap-1">
                  <Label className="text-xs font-medium">Copiar Archivos (.env)</Label>
                  <AutoExpandingTextarea
                    value={copyFiles}
                    onChange={(e) => setCopyFiles(e.target.value)}
                    placeholder="e.g. .env"
                    className="font-mono text-xs"
                    rows={1}
                  />
                </div>
              </div>
            </div>
          )}

          <DialogFooter className="gap-2">
            <Button variant="outline" onClick={handleCancel} disabled={isApplying}>
              {t('common:cancel', 'Cancelar')}
            </Button>
            <Button
              onClick={handleApply}
              disabled={isLoading || isApplying || !detected}
            >
              {isApplying && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
              <Sparkles className="mr-1.5 h-4 w-4" />
              Aplicar a este Repositorio
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }
);

export const AutoDetectScriptsDialog = defineModal<
  AutoDetectScriptsDialogProps,
  AutoDetectScriptsDialogResult
>(AutoDetectScriptsDialogImpl);
