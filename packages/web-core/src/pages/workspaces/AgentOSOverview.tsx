import { useCallback, useMemo, useState } from 'react';
import {
  ArrowRightIcon,
  ArrowUpRightIcon,
  CheckCircleIcon,
  FolderIcon,
  LightningIcon,
  PlusIcon,
  WarningCircleIcon,
} from '@phosphor-icons/react';
import { useAppNavigation } from '@/shared/hooks/useAppNavigation';
import { useIsMobile } from '@/shared/hooks/useIsMobile';
import { usePageTitle } from '@/shared/hooks/usePageTitle';
import { useWorkspaces, type Workspace } from '@/shared/hooks/useWorkspaces';
import { setCreateModeSeedState } from '@/features/create-mode/model/createModeSeedStore';
import './agentos-overview.css';

type WorkspaceState = 'running' | 'attention' | 'failed' | 'idle';

function isToday(value: string | undefined): boolean {
  if (!value) return false;

  const date = new Date(value);
  const now = new Date();
  return (
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate()
  );
}

function getWorkspaceState(workspace: Workspace): WorkspaceState {
  if (workspace.hasPendingApproval) return 'attention';
  if (workspace.isRunning) return 'running';
  if (workspace.latestProcessStatus === 'failed') return 'failed';
  return 'idle';
}

function getWorkspaceStateCopy(state: WorkspaceState): {
  label: string;
  tone: string;
} {
  switch (state) {
    case 'running':
      return { label: 'En ejecución', tone: 'running' };
    case 'attention':
      return { label: 'Requiere atención', tone: 'attention' };
    case 'failed':
      return { label: 'Último intento fallido', tone: 'failed' };
    case 'idle':
      return { label: 'En espera', tone: 'idle' };
  }
}

function formatRelativeTime(value: string): string {
  const elapsedMs = Date.now() - new Date(value).getTime();
  const elapsedMinutes = Math.max(0, Math.round(elapsedMs / 60_000));

  if (elapsedMinutes < 1) return 'ahora';
  if (elapsedMinutes < 60) return `hace ${elapsedMinutes} min`;

  const elapsedHours = Math.round(elapsedMinutes / 60);
  if (elapsedHours < 24) return `hace ${elapsedHours} h`;

  return new Intl.DateTimeFormat('es-ES', {
    day: 'numeric',
    month: 'short',
  }).format(new Date(value));
}

function WorkspaceRow({
  workspace,
  onOpen,
}: {
  workspace: Workspace;
  onOpen: (workspaceId: string) => void;
}) {
  const state = getWorkspaceState(workspace);
  const stateCopy = getWorkspaceStateCopy(state);

  return (
    <button
      type="button"
      className="agentos-workspace-row"
      onClick={() => onOpen(workspace.id)}
    >
      <span
        className={`agentos-status-dot agentos-status-dot--${stateCopy.tone}`}
      />
      <span className="agentos-workspace-row__body">
        <span className="agentos-workspace-row__title">{workspace.name}</span>
        <span className="agentos-workspace-row__meta">
          <span translate="no">{workspace.branch}</span>
          <span aria-hidden="true">·</span>
          <span>{stateCopy.label}</span>
        </span>
      </span>
      <span className="agentos-workspace-row__time">
        {formatRelativeTime(workspace.updatedAt)}
      </span>
      <ArrowUpRightIcon
        aria-hidden="true"
        className="agentos-workspace-row__arrow"
        size={16}
        weight="bold"
      />
    </button>
  );
}

export function AgentOSOverview() {
  const appNavigation = useAppNavigation();
  const isMobile = useIsMobile();
  const { workspaces, isLoading, isConnected, error } = useWorkspaces();
  const [objective, setObjective] = useState('');
  const [submitError, setSubmitError] = useState<string | null>(null);

  usePageTitle();

  const runningWorkspaces = useMemo(
    () => workspaces.filter((workspace) => workspace.isRunning),
    [workspaces]
  );
  const attentionWorkspaces = useMemo(
    () => workspaces.filter((workspace) => workspace.hasPendingApproval),
    [workspaces]
  );
  const completedToday = useMemo(
    () =>
      workspaces.filter(
        (workspace) =>
          workspace.latestProcessStatus === 'completed' &&
          isToday(workspace.latestProcessCompletedAt)
      ).length,
    [workspaces]
  );
  const recentWorkspaces = useMemo(
    () =>
      [...workspaces]
        .sort(
          (left, right) =>
            new Date(right.updatedAt).getTime() -
            new Date(left.updatedAt).getTime()
        )
        .slice(0, isMobile ? 5 : 8),
    [isMobile, workspaces]
  );
  const planeStatus = isLoading
    ? { label: 'Conectando…', tone: 'connecting' }
    : isConnected
      ? { label: 'Control plane online', tone: 'online' }
      : { label: 'Sin conexión', tone: 'offline' };

  const handleOpenWorkspace = useCallback(
    (workspaceId: string) => {
      appNavigation.goToWorkspace(workspaceId);
    },
    [appNavigation]
  );

  const handlePrepareExecution = useCallback(
    (event: React.FormEvent<HTMLFormElement>) => {
      event.preventDefault();
      const trimmedObjective = objective.trim();
      if (!trimmedObjective) {
        setSubmitError('Describe el objetivo antes de preparar la ejecución.');
        return;
      }

      setSubmitError(null);
      setCreateModeSeedState({ initialPrompt: trimmedObjective });
      appNavigation.goToWorkspacesCreate();
    },
    [appNavigation, objective]
  );

  return (
    <main className="agentos-overview" id="main-content" tabIndex={-1}>
      <a className="agentos-skip-link" href="#agentos-objective">
        Saltar al objetivo
      </a>

      <div className="agentos-overview__shell">
        <header className="agentos-overview__header">
          <div>
            <p className="agentos-eyebrow">CENTRO DE OPERACIONES</p>
            <h1>¿Qué quieres coordinar?</h1>
            <p className="agentos-overview__lede">
              Prepara una ejecución y supervisa tus agentes desde un mismo
              lugar.
            </p>
          </div>
          <div className="agentos-plane-status" aria-live="polite">
            <span
              className={`agentos-status-dot agentos-status-dot--${planeStatus.tone}`}
            />
            <span>{planeStatus.label}</span>
          </div>
        </header>

        <section className="agentos-objective-card" id="agentos-objective">
          <form onSubmit={handlePrepareExecution}>
            <label
              className="agentos-field-label"
              htmlFor="agentos-objective-input"
            >
              Objetivo de la ejecución
            </label>
            <textarea
              id="agentos-objective-input"
              name="objective"
              autoComplete="off"
              spellCheck
              value={objective}
              onChange={(event) => {
                setObjective(event.target.value);
                if (submitError) setSubmitError(null);
              }}
              placeholder="Describe qué quieres conseguir…"
              rows={isMobile ? 4 : 3}
              aria-describedby={
                submitError ? 'agentos-objective-error' : undefined
              }
            />
            <div className="agentos-objective-card__footer">
              <div
                className="agentos-context"
                aria-label="Contexto de ejecución"
              >
                <span className="agentos-context__label">Se preparará con</span>
                <span className="agentos-context__chip">
                  <FolderIcon aria-hidden="true" size={14} />
                  Workspace local
                </span>
                <span className="agentos-context__chip">
                  <LightningIcon aria-hidden="true" size={14} />
                  Agente recomendado
                </span>
              </div>
              <button className="agentos-primary-button" type="submit">
                Preparar ejecución
                <ArrowRightIcon aria-hidden="true" size={16} weight="bold" />
              </button>
            </div>
            {submitError && (
              <p
                className="agentos-inline-error"
                id="agentos-objective-error"
                role="alert"
              >
                {submitError}
              </p>
            )}
          </form>
          <p className="agentos-objective-card__hint">
            El siguiente paso te permitirá elegir repositorio, rama, agente y
            modelo antes de lanzar nada.
          </p>
        </section>

        <section className="agentos-metrics" aria-label="Resumen operativo">
          <article className="agentos-metric-card">
            <span className="agentos-metric-card__label">En ejecución</span>
            <strong>{runningWorkspaces.length}</strong>
            <span className="agentos-metric-card__detail">
              workspaces activos
            </span>
          </article>
          <article className="agentos-metric-card">
            <span className="agentos-metric-card__label">
              Requieren atención
            </span>
            <strong>{attentionWorkspaces.length}</strong>
            <span className="agentos-metric-card__detail">
              aprobaciones pendientes
            </span>
          </article>
          <article className="agentos-metric-card">
            <span className="agentos-metric-card__label">Completadas hoy</span>
            <strong>{completedToday}</strong>
            <span className="agentos-metric-card__detail">
              último proceso correcto
            </span>
          </article>
          <article className="agentos-metric-card">
            <span className="agentos-metric-card__label">Control plane</span>
            <strong className="agentos-metric-card__status">
              <span
                className={`agentos-status-dot agentos-status-dot--${planeStatus.tone}`}
              />
              {isLoading ? 'Conectando…' : isConnected ? 'Online' : 'Offline'}
            </strong>
            <span className="agentos-metric-card__detail">streams locales</span>
          </article>
        </section>

        <section className="agentos-content-grid">
          <div className="agentos-panel agentos-panel--workspaces">
            <div className="agentos-panel__header">
              <div>
                <p className="agentos-eyebrow">SUPERVISIÓN</p>
                <h2>Workspaces recientes</h2>
              </div>
              <button
                type="button"
                className="agentos-secondary-button"
                onClick={() => appNavigation.goToWorkspacesCreate()}
              >
                <PlusIcon aria-hidden="true" size={15} weight="bold" />
                Nuevo
              </button>
            </div>

            {isLoading ? (
              <p className="agentos-panel__empty" aria-live="polite">
                Cargando workspaces…
              </p>
            ) : recentWorkspaces.length === 0 ? (
              <div className="agentos-panel__empty">
                <CheckCircleIcon aria-hidden="true" size={22} />
                <p>Aún no hay workspaces activos.</p>
                <span>Prepara una ejecución para crear el primero.</span>
              </div>
            ) : (
              <div className="agentos-workspace-list">
                {recentWorkspaces.map((workspace) => (
                  <WorkspaceRow
                    key={workspace.id}
                    workspace={workspace}
                    onOpen={handleOpenWorkspace}
                  />
                ))}
              </div>
            )}
          </div>

          <aside className="agentos-panel agentos-panel--activity">
            <div className="agentos-panel__header">
              <div>
                <p className="agentos-eyebrow">SEÑALES</p>
                <h2>Estado de la operación</h2>
              </div>
            </div>
            <div className="agentos-signal-list">
              <div className="agentos-signal">
                <span className="agentos-signal__icon agentos-signal__icon--green">
                  <CheckCircleIcon aria-hidden="true" size={16} weight="fill" />
                </span>
                <div>
                  <strong>Persistencia local</strong>
                  <span>Estados y sesiones viven en tu instalación.</span>
                </div>
              </div>
              <div className="agentos-signal">
                <span className="agentos-signal__icon agentos-signal__icon--amber">
                  <WarningCircleIcon
                    aria-hidden="true"
                    size={16}
                    weight="fill"
                  />
                </span>
                <div>
                  <strong>Control humano</strong>
                  <span>
                    {attentionWorkspaces.length > 0
                      ? `${attentionWorkspaces.length} aprobación${attentionWorkspaces.length === 1 ? '' : 'es'} pendiente${attentionWorkspaces.length === 1 ? '' : 's'}.`
                      : 'No hay aprobaciones pendientes.'}
                  </span>
                </div>
              </div>
              <div className="agentos-signal">
                <span className="agentos-signal__icon agentos-signal__icon--neutral">
                  <LightningIcon aria-hidden="true" size={16} weight="fill" />
                </span>
                <div>
                  <strong>Routing</strong>
                  <span>La selección automática aún no está activada.</span>
                </div>
              </div>
            </div>
            {error && (
              <p
                className="agentos-inline-error agentos-inline-error--panel"
                role="alert"
              >
                No se pudo actualizar el stream de workspaces. Comprueba el
                servidor local antes de iniciar una tarea.
              </p>
            )}
          </aside>
        </section>

        <footer className="agentos-overview__footer">
          <span>AgentOS · supervisión local-first</span>
          <span>
            Los estados reflejan eventos observados, no inferencias de pantalla.
          </span>
        </footer>
      </div>
    </main>
  );
}
