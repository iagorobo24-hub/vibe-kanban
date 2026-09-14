import { useCallback, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
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

function getWorkspaceStateCopy(
  state: WorkspaceState,
  translate: (key: string) => string
): {
  label: string;
  tone: string;
} {
  switch (state) {
    case 'running':
      return {
        label: translate('agentosOverview.states.running'),
        tone: 'running',
      };
    case 'attention':
      return {
        label: translate('agentosOverview.states.attention'),
        tone: 'attention',
      };
    case 'failed':
      return {
        label: translate('agentosOverview.states.failed'),
        tone: 'failed',
      };
    case 'idle':
      return { label: translate('agentosOverview.states.idle'), tone: 'idle' };
  }
}

function formatRelativeTime(
  value: string,
  translate: (key: string, options?: { count: number }) => string,
  locale: string
): string {
  const elapsedMs = Date.now() - new Date(value).getTime();
  const elapsedMinutes = Math.max(0, Math.round(elapsedMs / 60_000));

  if (elapsedMinutes < 1) return translate('agentosOverview.relative.now');
  if (elapsedMinutes < 60) {
    return translate('agentosOverview.relative.minutes', {
      count: elapsedMinutes,
    });
  }

  const elapsedHours = Math.round(elapsedMinutes / 60);
  if (elapsedHours < 24) {
    return translate('agentosOverview.relative.hours', { count: elapsedHours });
  }

  return new Intl.DateTimeFormat(locale, {
    day: 'numeric',
    month: 'short',
  }).format(new Date(value));
}

function WorkspaceRow({
  workspace,
  onOpen,
  translate,
  locale,
}: {
  workspace: Workspace;
  onOpen: (workspaceId: string) => void;
  translate: (key: string) => string;
  locale: string;
}) {
  const state = getWorkspaceState(workspace);
  const stateCopy = getWorkspaceStateCopy(state, translate);

  return (
    <button
      type="button"
      className="agentos-workspace-row"
      onClick={() => onOpen(workspace.id)}
    >
      <span
        aria-hidden="true"
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
        {formatRelativeTime(workspace.updatedAt, translate, locale)}
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
  const { t, i18n } = useTranslation('common');
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
    ? { label: t('agentosOverview.status.connecting'), tone: 'connecting' }
    : isConnected
      ? { label: t('agentosOverview.status.online'), tone: 'online' }
      : { label: t('agentosOverview.status.offline'), tone: 'offline' };
  const streamUnavailable = !isLoading && (!isConnected || Boolean(error));
  const metricsUnavailable = isLoading || streamUnavailable;
  const locale = i18n.resolvedLanguage === 'es' ? 'es-ES' : 'en-US';

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
        setSubmitError(t('agentosOverview.objective.emptyError'));
        return;
      }

      setSubmitError(null);
      setCreateModeSeedState({ initialPrompt: trimmedObjective });
      appNavigation.goToWorkspacesCreate();
    },
    [appNavigation, objective, t]
  );

  return (
    <main className="agentos-overview" id="main-content" tabIndex={-1}>
      <a className="agentos-skip-link" href="#agentos-objective">
        {t('agentosOverview.skipToObjective')}
      </a>

      <div className="agentos-overview__shell">
        <header className="agentos-overview__header">
          <div>
            <p className="agentos-eyebrow">{t('agentosOverview.eyebrow')}</p>
            <h1>{t('agentosOverview.title')}</h1>
            <p className="agentos-overview__lede">
              {t('agentosOverview.lede')}
            </p>
          </div>
          <div className="agentos-plane-status" aria-live="polite">
            <span
              aria-hidden="true"
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
              {t('agentosOverview.objective.label')}
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
              placeholder={t('agentosOverview.objective.placeholder')}
              rows={isMobile ? 4 : 3}
              aria-describedby={
                submitError ? 'agentos-objective-error' : undefined
              }
            />
            <div className="agentos-objective-card__footer">
              <div
                className="agentos-context"
                aria-label={t('agentosOverview.objective.contextAriaLabel')}
                role="group"
              >
                <span className="agentos-context__label">
                  {t('agentosOverview.objective.contextLabel')}
                </span>
                <span className="agentos-context__chip">
                  <FolderIcon aria-hidden="true" size={14} />
                  {t('agentosOverview.objective.localWorkspace')}
                </span>
                <span className="agentos-context__chip">
                  <LightningIcon aria-hidden="true" size={14} />
                  {t('agentosOverview.objective.recommendedAgent')}
                </span>
              </div>
              <button className="agentos-primary-button" type="submit">
                {t('agentosOverview.objective.submit')}
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
            {t('agentosOverview.objective.hint')}
          </p>
        </section>

        <section
          className="agentos-metrics"
          aria-label={t('agentosOverview.metrics.ariaLabel')}
        >
          <article className="agentos-metric-card">
            <span className="agentos-metric-card__label">
              {t('agentosOverview.metrics.running')}
            </span>
            <strong
              aria-label={
                metricsUnavailable
                  ? t('agentosOverview.metrics.unavailable')
                  : undefined
              }
            >
              {metricsUnavailable
                ? t('agentosOverview.metrics.unavailable')
                : runningWorkspaces.length}
            </strong>
            <span className="agentos-metric-card__detail">
              {metricsUnavailable
                ? t('agentosOverview.metrics.unavailableDetail')
                : t('agentosOverview.metrics.runningDetail')}
            </span>
          </article>
          <article className="agentos-metric-card">
            <span className="agentos-metric-card__label">
              {t('agentosOverview.metrics.attention')}
            </span>
            <strong
              aria-label={
                metricsUnavailable
                  ? t('agentosOverview.metrics.unavailable')
                  : undefined
              }
            >
              {metricsUnavailable
                ? t('agentosOverview.metrics.unavailable')
                : attentionWorkspaces.length}
            </strong>
            <span className="agentos-metric-card__detail">
              {metricsUnavailable
                ? t('agentosOverview.metrics.unavailableDetail')
                : t('agentosOverview.metrics.attentionDetail')}
            </span>
          </article>
          <article className="agentos-metric-card">
            <span className="agentos-metric-card__label">
              {t('agentosOverview.metrics.completedToday')}
            </span>
            <strong
              aria-label={
                metricsUnavailable
                  ? t('agentosOverview.metrics.unavailable')
                  : undefined
              }
            >
              {metricsUnavailable
                ? t('agentosOverview.metrics.unavailable')
                : completedToday}
            </strong>
            <span className="agentos-metric-card__detail">
              {metricsUnavailable
                ? t('agentosOverview.metrics.unavailableDetail')
                : t('agentosOverview.metrics.completedDetail')}
            </span>
          </article>
          <article className="agentos-metric-card">
            <span className="agentos-metric-card__label">
              {t('agentosOverview.metrics.controlPlane')}
            </span>
            <strong className="agentos-metric-card__status">
              <span
                aria-hidden="true"
                className={`agentos-status-dot agentos-status-dot--${planeStatus.tone}`}
              />
              {isLoading
                ? t('agentosOverview.status.connecting')
                : isConnected
                  ? t('agentosOverview.status.onlineShort')
                  : t('agentosOverview.status.offlineShort')}
            </strong>
            <span className="agentos-metric-card__detail">
              {metricsUnavailable
                ? t('agentosOverview.metrics.unavailableDetail')
                : t('agentosOverview.metrics.streams')}
            </span>
          </article>
        </section>

        <section className="agentos-content-grid">
          <div className="agentos-panel agentos-panel--workspaces">
            <div className="agentos-panel__header">
              <div>
                <p className="agentos-eyebrow">
                  {t('agentosOverview.supervision')}
                </p>
                <h2>{t('agentosOverview.recentWorkspaces')}</h2>
              </div>
              <button
                type="button"
                className="agentos-secondary-button"
                onClick={() => appNavigation.goToWorkspacesCreate()}
              >
                <PlusIcon aria-hidden="true" size={15} weight="bold" />
                {t('agentosOverview.newWorkspace')}
              </button>
            </div>

            {isLoading ? (
              <p className="agentos-panel__empty" aria-live="polite">
                {t('agentosOverview.loadingWorkspaces')}
              </p>
            ) : streamUnavailable ? (
              <div className="agentos-panel__empty agentos-panel__empty--error">
                <WarningCircleIcon aria-hidden="true" size={22} />
                <p>{t('agentosOverview.streamUnavailableTitle')}</p>
                <span>{t('agentosOverview.streamError')}</span>
              </div>
            ) : recentWorkspaces.length === 0 ? (
              <div className="agentos-panel__empty">
                <CheckCircleIcon aria-hidden="true" size={22} />
                <p>{t('agentosOverview.emptyTitle')}</p>
                <span>{t('agentosOverview.emptyDescription')}</span>
              </div>
            ) : (
              <div className="agentos-workspace-list">
                {recentWorkspaces.map((workspace) => (
                  <WorkspaceRow
                    key={workspace.id}
                    workspace={workspace}
                    onOpen={handleOpenWorkspace}
                    translate={t}
                    locale={locale}
                  />
                ))}
              </div>
            )}
          </div>

          <aside className="agentos-panel agentos-panel--activity">
            <div className="agentos-panel__header">
              <div>
                <p className="agentos-eyebrow">
                  {t('agentosOverview.signals')}
                </p>
                <h2>{t('agentosOverview.operationStatus')}</h2>
              </div>
            </div>
            <div className="agentos-signal-list">
              <div className="agentos-signal">
                <span className="agentos-signal__icon agentos-signal__icon--green">
                  <CheckCircleIcon aria-hidden="true" size={16} weight="fill" />
                </span>
                <div>
                  <strong>{t('agentosOverview.persistence')}</strong>
                  <span>{t('agentosOverview.persistenceDetail')}</span>
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
                  <strong>{t('agentosOverview.humanControl')}</strong>
                  <span>
                    {attentionWorkspaces.length > 0
                      ? t('agentosOverview.pendingApprovals', {
                          count: attentionWorkspaces.length,
                        })
                      : t('agentosOverview.noPendingApprovals')}
                  </span>
                </div>
              </div>
              <div className="agentos-signal">
                <span className="agentos-signal__icon agentos-signal__icon--neutral">
                  <LightningIcon aria-hidden="true" size={16} weight="fill" />
                </span>
                <div>
                  <strong>{t('agentosOverview.routing')}</strong>
                  <span>{t('agentosOverview.routingDetail')}</span>
                </div>
              </div>
            </div>
            {error && (
              <p
                className="agentos-inline-error agentos-inline-error--panel"
                role="alert"
              >
                {t('agentosOverview.streamError')}
              </p>
            )}
          </aside>
        </section>

        <footer className="agentos-overview__footer">
          <span>{t('agentosOverview.footerLabel')}</span>
          <span>{t('agentosOverview.footerEvidence')}</span>
        </footer>
      </div>
    </main>
  );
}
