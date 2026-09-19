import { useState, useEffect, useCallback } from 'react';
import { create, useModal } from '@ebay/nice-modal-react';
import { useQueryClient } from '@tanstack/react-query';
import {
  Loader2,
  Palette,
  CheckCircle,
  AlertCircle,
  Sparkles,
  Save,
  RefreshCw,
} from 'lucide-react';
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
import { defineModal, getErrorMessage } from '@/shared/lib/modals';
import { designApi } from '@/shared/lib/api';
import type {
  DesignSystemTokens,
  AuditDesignSystemResult,
} from 'shared/types';

export interface DesignStudioDialogProps {
  workspaceId: string;
  workspaceName?: string;
}

export type DesignStudioDialogResult = {
  action: 'closed';
};

type TabId = 'tokens' | 'generator' | 'audit' | 'editor';

const TABS: Array<{ id: TabId; label: string }> = [
  { id: 'tokens', label: 'Tokens' },
  { id: 'generator', label: 'Generador' },
  { id: 'audit', label: 'Auditoría' },
  { id: 'editor', label: 'Editor' },
];

function Swatch({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-center gap-2">
      <span
        className="h-6 w-6 rounded-sm border border-border/60"
        style={{ backgroundColor: value }}
        title={value}
      />
      <div className="min-w-0">
        <div className="text-xs text-low">{label}</div>
        <div className="truncate font-mono text-xs text-normal">{value}</div>
      </div>
    </div>
  );
}

function Field({ label, value }: { label: string; value?: string | null }) {
  if (!value) return null;
  return (
    <div>
      <div className="text-xs text-low">{label}</div>
      <div className="text-sm text-normal">{value}</div>
    </div>
  );
}

const DesignStudioDialogImpl = create<DesignStudioDialogProps>(
  ({ workspaceId, workspaceName }) => {
    const modal = useModal();
    const queryClient = useQueryClient();

    const [tab, setTab] = useState<TabId>('tokens');
    const [tokens, setTokens] = useState<DesignSystemTokens | null>(null);
    const [isLoading, setIsLoading] = useState(true);
    const [error, setError] = useState<string | null>(null);

    const [prompt, setPrompt] = useState('');
    const [projectName, setProjectName] = useState('');
    const [isGenerating, setIsGenerating] = useState(false);

    const [audit, setAudit] = useState<AuditDesignSystemResult | null>(null);
    const [isAuditing, setIsAuditing] = useState(false);

    const [markdown, setMarkdown] = useState('');
    const [isSaving, setIsSaving] = useState(false);

    const reload = useCallback(async () => {
      setIsLoading(true);
      setError(null);
      try {
        const loaded = await designApi.get(workspaceId);
        setTokens(loaded);
        setMarkdown(loaded?.raw_markdown ?? '');
      } catch (err) {
        setError(getErrorMessage(err));
      } finally {
        setIsLoading(false);
      }
    }, [workspaceId]);

    useEffect(() => {
      void reload();
    }, [reload]);

    const handleGenerate = useCallback(async () => {
      if (!prompt.trim() || isGenerating) return;
      setIsGenerating(true);
      setError(null);
      try {
        const generated = await designApi.generate(workspaceId, {
          prompt: prompt.trim(),
          project_name: projectName.trim() || null,
          stack: null,
        });
        setTokens(generated);
        setMarkdown(generated.raw_markdown);
        setTab('tokens');
        queryClient.invalidateQueries({ queryKey: ['workspace', workspaceId] });
      } catch (err) {
        setError(getErrorMessage(err));
      } finally {
        setIsGenerating(false);
      }
    }, [workspaceId, prompt, projectName, isGenerating, queryClient]);

    const handleAudit = useCallback(async () => {
      if (isAuditing) return;
      setIsAuditing(true);
      setError(null);
      try {
        setAudit(await designApi.audit(workspaceId));
      } catch (err) {
        setError(getErrorMessage(err));
      } finally {
        setIsAuditing(false);
      }
    }, [workspaceId, isAuditing]);

    const handleSave = useCallback(async () => {
      if (isSaving) return;
      setIsSaving(true);
      setError(null);
      try {
        const saved = await designApi.save(workspaceId, markdown);
        setTokens(saved);
        setMarkdown(saved.raw_markdown);
        setTab('tokens');
      } catch (err) {
        setError(getErrorMessage(err));
      } finally {
        setIsSaving(false);
      }
    }, [workspaceId, markdown, isSaving]);

    const handleClose = useCallback(() => {
      modal.resolve({ action: 'closed' });
      modal.hide();
    }, [modal]);

    return (
      <Dialog open={modal.visible} onOpenChange={(open) => !open && handleClose()}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2 text-base font-semibold">
              <Palette className="h-5 w-5 text-brand" />
              Modo Design {workspaceName ? `— ${workspaceName}` : ''}
            </DialogTitle>
          </DialogHeader>

          <div className="flex gap-1 border-b border-border/60">
            {TABS.map((t) => (
              <button
                key={t.id}
                type="button"
                onClick={() => setTab(t.id)}
                className={`px-3 py-1.5 text-sm font-medium ${
                  tab === t.id
                    ? 'border-b-2 border-brand text-normal'
                    : 'text-low hover:text-normal'
                }`}
              >
                {t.label}
              </button>
            ))}
          </div>

          {isLoading ? (
            <div className="flex flex-col items-center justify-center py-8 gap-2 text-sm text-low">
              <Loader2 className="h-6 w-6 animate-spin text-brand" />
              <span>Cargando sistema de diseño...</span>
            </div>
          ) : error ? (
            <div className="rounded-sm border border-error/30 bg-error/10 p-3 text-sm text-error">
              {error}
            </div>
          ) : (
            <div className="py-2">
              {tab === 'tokens' &&
                (tokens ? (
                  <div className="flex flex-col gap-4">
                    <div>
                      <div className="text-xs text-low">Estilo activo</div>
                      <div className="text-base font-semibold text-normal">
                        {tokens.style.name}
                      </div>
                      <div className="mt-2 grid grid-cols-2 gap-2">
                        <Field label="Keywords" value={tokens.style.keywords} />
                        <Field label="Ideal para" value={tokens.style.best_for} />
                        <Field label="Rendimiento" value={tokens.style.performance} />
                        <Field label="Accesibilidad" value={tokens.style.accessibility} />
                      </div>
                    </div>
                    <div>
                      <div className="mb-1 text-xs text-low">Paleta</div>
                      <div className="grid grid-cols-2 gap-2 md:grid-cols-3">
                        <Swatch label="Primary" value={tokens.colors.primary} />
                        <Swatch label="Secondary" value={tokens.colors.secondary} />
                        <Swatch label="CTA" value={tokens.colors.cta} />
                        <Swatch label="Background" value={tokens.colors.background} />
                        <Swatch label="Texto" value={tokens.colors.text} />
                      </div>
                      <Field label="Notas" value={tokens.colors.notes} />
                    </div>
                    <div>
                      <div className="mb-1 text-xs text-low">Tipografía</div>
                      <div className="grid grid-cols-2 gap-2">
                        <Field label="Titulares" value={tokens.typography.heading} />
                        <Field label="Cuerpo" value={tokens.typography.body} />
                        <Field label="Tono" value={tokens.typography.mood} />
                        <Field label="Ideal para" value={tokens.typography.best_for} />
                        <Field label="Google Fonts" value={tokens.typography.google_fonts} />
                        <Field label="CSS import" value={tokens.typography.css_import} />
                      </div>
                    </div>
                    <Field label="Efectos clave" value={tokens.key_effects} />
                    {tokens.avoid_anti_patterns.length > 0 && (
                      <div>
                        <div className="mb-1 text-xs text-low">Evitar</div>
                        <ul className="list-disc pl-4 text-sm text-normal">
                          {tokens.avoid_anti_patterns.map((a, i) => (
                            <li key={i}>{a}</li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </div>
                ) : (
                  <div className="py-6 text-center text-sm text-low">
                    Sin sistema de diseño. Genéralo en la pestaña Generador o
                    pega tu MASTER.md en Editor.
                  </div>
                ))}

              {tab === 'generator' && (
                <div className="flex flex-col gap-3">
                  <div className="flex flex-col gap-1.5">
                    <Label className="text-xs font-medium">Describe la interfaz</Label>
                    <AutoExpandingTextarea
                      value={prompt}
                      onChange={(e) => setPrompt(e.target.value)}
                      placeholder="ej. Fintech dashboard con modo oscuro"
                      rows={3}
                    />
                  </div>
                  <div className="flex flex-col gap-1.5">
                    <Label className="text-xs font-medium">Proyecto (opcional)</Label>
                    <AutoExpandingTextarea
                      value={projectName}
                      onChange={(e) => setProjectName(e.target.value)}
                      placeholder="Nombre del proyecto"
                      rows={1}
                    />
                  </div>
                  <div>
                    <Button onClick={handleGenerate} disabled={!prompt.trim() || isGenerating}>
                      {isGenerating && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                      <Sparkles className="mr-1.5 h-4 w-4" />
                      Generar Sistema de Diseño
                    </Button>
                  </div>
                </div>
              )}

              {tab === 'audit' && (
                <div className="flex flex-col gap-3">
                  <div>
                    <Button
                      variant="outline"
                      onClick={handleAudit}
                      disabled={isAuditing}
                    >
                      {isAuditing ? (
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      ) : (
                        <RefreshCw className="mr-1.5 h-4 w-4" />
                      )}
                      Re-auditar
                    </Button>
                  </div>
                  {tokens && tokens.checklist.length > 0 && (
                    <div>
                      <div className="mb-1 text-xs text-low">Checklist pre-entrega</div>
                      <ul className="flex flex-col gap-1">
                        {tokens.checklist.map((c, i) => (
                          <li key={i} className="flex items-center gap-2 text-sm text-normal">
                            {c.passed ? (
                              <CheckCircle className="h-4 w-4 text-success" />
                            ) : (
                              <AlertCircle className="h-4 w-4 text-amber-500" />
                            )}
                            {c.rule}
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                  {audit && (
                    <div>
                      <div className="mb-1 text-xs text-low">
                        Violaciones ({audit.total_violations}) —{' '}
                        {audit.compliant ? 'conforme' : 'no conforme'}
                      </div>
                      {audit.violations.length === 0 ? (
                        <div className="text-sm text-low">Sin violaciones.</div>
                      ) : (
                        <ul className="flex flex-col gap-1.5">
                          {audit.violations.map((v, i) => (
                            <li
                              key={i}
                              className="rounded-sm border border-border/60 p-2 text-sm"
                            >
                              <div className="font-mono text-xs text-low">
                                {v.file}
                                {v.line != null ? `:${v.line}` : ''} — {v.rule}
                              </div>
                              <div className="text-normal">{v.message}</div>
                            </li>
                          ))}
                        </ul>
                      )}
                    </div>
                  )}
                </div>
              )}

              {tab === 'editor' && (
                <div className="flex flex-col gap-2">
                  <Label className="text-xs font-medium">MASTER.md</Label>
                  <AutoExpandingTextarea
                    value={markdown}
                    onChange={(e) => setMarkdown(e.target.value)}
                    placeholder="# Design System: ..."
                    className="font-mono text-xs"
                    rows={14}
                  />
                  <div>
                    <Button onClick={handleSave} disabled={isSaving || !markdown.trim()}>
                      {isSaving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
                      <Save className="mr-1.5 h-4 w-4" />
                      Guardar MASTER.md
                    </Button>
                  </div>
                </div>
              )}
            </div>
          )}

          <DialogFooter className="gap-2">
            <Button variant="outline" onClick={handleClose}>
              Cerrar
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    );
  }
);

export const DesignStudioDialog = defineModal<
  DesignStudioDialogProps,
  DesignStudioDialogResult
>(DesignStudioDialogImpl);
