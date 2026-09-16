import { useCallback, useMemo, useState } from 'react';
import { useQueryClient } from '@tanstack/react-query';
import {
  ClockCounterClockwiseIcon,
  GitBranchIcon,
  MagnifyingGlassIcon,
  PlusIcon,
  SparkleIcon,
  SpinnerIcon,
  XIcon,
} from '@phosphor-icons/react';
import { useTranslation } from 'react-i18next';
import type { Repo } from 'shared/types';
import type { BranchItem, RepoItem } from '@/shared/types/selectionItems';
import { repoApi } from '@/shared/lib/api';
import { cn } from '@/shared/lib/utils';
import { useCreateMode } from '@/features/create-mode/model/useCreateMode';
import { FolderPickerDialog } from '@/shared/dialogs/shared/FolderPickerDialog';
import { SettingsDialog } from '@/shared/dialogs/settings/SettingsDialog';
import { AutoDetectScriptsDialog } from '@/shared/dialogs/scripts/AutoDetectScriptsDialog';
import { PrimaryButton } from '@vibe/ui/components/PrimaryButton';
import { CreateRepoDialog } from '@vibe/ui/components/CreateRepoDialog';
import {
  SelectionDialog,
  type SelectionPage,
} from '@/shared/dialogs/command-bar/SelectionDialog';
import {
  buildRepoSelectionPages,
  type RepoSelectionResult,
} from '@/shared/dialogs/command-bar/selections/repoSelection';
import {
  buildBranchSelectionPages,
  type BranchSelectionResult,
} from '@/shared/dialogs/command-bar/selections/branchSelection';

function toRepoItem(repo: Repo): RepoItem {
  return {
    id: repo.id,
    display_name: repo.display_name || repo.name,
  };
}

function toBranchItem(branch: {
  name: string;
  is_current: boolean;
}): BranchItem {
  return {
    name: branch.name,
    isCurrent: branch.is_current,
  };
}

function getRepoDisplayName(repo: Repo): string {
  return repo.display_name || repo.name;
}

type PendingAction = 'choose' | 'browse' | 'create' | 'branch' | null;

const inlineControlButtonClassName =
  'inline-flex items-center gap-half rounded-sm px-half py-half text-sm text-normal ' +
  'hover:text-high focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand disabled:cursor-not-allowed disabled:opacity-50';

const recentInlineControlButtonClassName =
  'inline-flex items-center gap-half rounded-sm px-half py-half text-sm ' +
  'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand disabled:cursor-not-allowed disabled:opacity-50';

const repoRowButtonClassName =
  'inline-flex items-center gap-half text-sm text-low hover:text-high ' +
  'rounded-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand disabled:cursor-not-allowed disabled:opacity-50';

interface CreateModeRepoPickerBarProps {
  onContinueToPrompt: () => void;
}

export function CreateModeRepoPickerBar({
  onContinueToPrompt,
}: CreateModeRepoPickerBarProps) {
  const { t } = useTranslation('common');
  const queryClient = useQueryClient();
  const { repos, targetBranches, addRepo, removeRepo, setTargetBranch } =
    useCreateMode();
  const [pendingAction, setPendingAction] = useState<PendingAction>(null);
  const [branchRepoId, setBranchRepoId] = useState<string | null>(null);
  const [pickerError, setPickerError] = useState<string | null>(null);
  const [setupHintDismissed, setSetupHintDismissed] = useState(false);
  const isBusy = pendingAction !== null;

  const hasUnconfiguredRepo = useMemo(
    () => repos.some((repo) => !repo.setup_script),
    [repos]
  );
  const showSetupHint = hasUnconfiguredRepo && !setupHintDismissed;

  const selectedRepoIds = useMemo(
    () => new Set(repos.map((repo) => repo.id)),
    [repos]
  );

  const pickBranchForRepo = useCallback(async (repo: Repo) => {
    const branches = await repoApi.getBranches(repo.id);
    const branchItems = branches.map(toBranchItem);
    const branchResult = (await SelectionDialog.show({
      initialPageId: 'selectBranch',
      pages: buildBranchSelectionPages(
        branchItems,
        getRepoDisplayName(repo)
      ) as Record<string, SelectionPage>,
    })) as BranchSelectionResult | undefined;

    return branchResult?.branch ?? null;
  }, []);

  const runPickerAction = useCallback(
    async (
      action: Exclude<PendingAction, null>,
      run: () => Promise<void>,
      fallbackError: string
    ) => {
      setPickerError(null);
      setPendingAction(action);

      try {
        await run();
      } catch (error) {
        setPickerError(error instanceof Error ? error.message : fallbackError);
      } finally {
        setPendingAction(null);
        if (action === 'branch') {
          setBranchRepoId(null);
        }
      }
    },
    []
  );

  const addRepoWithBranchSelection = useCallback(
    async (repo: Repo) => {
      if (selectedRepoIds.has(repo.id)) {
        setPickerError(
          t('createMode.repoPicker.errors.repositoryAlreadySelected')
        );
        return false;
      }

      const selectedBranch = await pickBranchForRepo(repo);
      if (!selectedBranch) return false;

      addRepo(repo);
      setTargetBranch(repo.id, selectedBranch);
      return true;
    },
    [addRepo, pickBranchForRepo, selectedRepoIds, setTargetBranch, t]
  );

  const handleChooseRepo = useCallback(async () => {
    await runPickerAction(
      'choose',
      async () => {
        const allRepos = await repoApi.listRecent();
        const availableRepos = allRepos.filter(
          (repo) => !selectedRepoIds.has(repo.id)
        );

        if (availableRepos.length === 0) {
          setPickerError(
            t('createMode.repoPicker.errors.noRecentRepositories')
          );
          return;
        }

        const repoResult = (await SelectionDialog.show({
          initialPageId: 'selectRepo',
          pages: buildRepoSelectionPages(
            availableRepos.map(toRepoItem)
          ) as Record<string, SelectionPage>,
        })) as RepoSelectionResult | undefined;

        if (!repoResult?.repoId) return;

        const selectedRepo = availableRepos.find(
          (repo) => repo.id === repoResult.repoId
        );
        if (!selectedRepo) return;

        await addRepoWithBranchSelection(selectedRepo);
      },
      t('createMode.repoPicker.errors.loadRepositories')
    );
  }, [addRepoWithBranchSelection, runPickerAction, selectedRepoIds, t]);

  const handleBrowseRepo = useCallback(async () => {
    await runPickerAction(
      'browse',
      async () => {
        const selectedPath = await FolderPickerDialog.show({
          title: t('dialogs.selectGitRepository'),
          description: t('dialogs.chooseExistingRepo'),
        });
        if (!selectedPath) return;

        const repo = await repoApi.register({ path: selectedPath });
        queryClient.invalidateQueries({ queryKey: ['repos'] });
        await addRepoWithBranchSelection(repo);
      },
      t('createMode.repoPicker.errors.registerRepository')
    );
  }, [addRepoWithBranchSelection, runPickerAction, t]);

  const handleCreateRepo = useCallback(async () => {
    await runPickerAction(
      'create',
      async () => {
        await CreateRepoDialog.show({
          onBrowseForPath: async (currentPath) =>
            FolderPickerDialog.show({
              title: t('git.createRepo.browseDialog.title'),
              description: t('git.createRepo.browseDialog.description'),
              value: currentPath,
            }),
          onCreateRepo: async ({ parentPath, folderName }) => {
            const repo = await repoApi.init({
              parent_path: parentPath,
              folder_name: folderName,
            });
            queryClient.invalidateQueries({ queryKey: ['repos'] });
            await addRepoWithBranchSelection(repo);
          },
        });
      },
      t('createMode.repoPicker.errors.createRepository')
    );
  }, [addRepoWithBranchSelection, runPickerAction, t]);

  const handleChangeBranch = useCallback(
    async (repo: Repo) => {
      setBranchRepoId(repo.id);
      await runPickerAction(
        'branch',
        async () => {
          const selectedBranch = await pickBranchForRepo(repo);
          if (!selectedBranch) return;
          setTargetBranch(repo.id, selectedBranch);
        },
        t('createMode.repoPicker.errors.loadBranches')
      );
    },
    [pickBranchForRepo, runPickerAction, setTargetBranch, t]
  );

  return (
    <div className="agentos-create-workspace__repo-picker w-chat max-w-full">
      <div className="px-plusfifty py-base">
        {repos.length > 0 && (
          <div>
            <div className="agentos-create-workspace__repo-list rounded-sm border border-border/60">
              {repos.map((repo, index) => {
                const branch =
                  targetBranches[repo.id] ??
                  t('createMode.repoSummary.selectBranch');
                const repoDisplayName = getRepoDisplayName(repo);
                const isChangingBranch =
                  pendingAction === 'branch' && branchRepoId === repo.id;

                return (
                  <div
                    key={repo.id}
                    className={cn(
                      'agentos-create-workspace__repo-row flex min-w-0 items-center gap-half px-base py-half',
                      index > 0 && 'border-t border-border/60'
                    )}
                  >
                    <span className="min-w-0 flex-1 truncate text-sm text-normal">
                      {repoDisplayName}
                    </span>
                    <span className="h-3 w-px shrink-0 bg-border/70" />
                    <button
                      type="button"
                      onClick={() => handleChangeBranch(repo)}
                      disabled={isBusy}
                      className={repoRowButtonClassName}
                      title={t('createMode.repoPicker.changeBranch')}
                    >
                      {isChangingBranch ? (
                        <SpinnerIcon
                          className="size-icon-xs animate-spin"
                          aria-hidden="true"
                        />
                      ) : (
                        <GitBranchIcon
                          className="size-icon-xs"
                          weight="bold"
                          aria-hidden="true"
                        />
                      )}
                      <span className="max-w-[200px] truncate">{branch}</span>
                    </button>
                    <span className="h-3 w-px shrink-0 bg-border/70" />
                    <button
                      type="button"
                      onClick={() => removeRepo(repo.id)}
                      disabled={isBusy}
                      aria-label={t('createMode.repoPicker.removeRepository', {
                        repository: repoDisplayName,
                      })}
                      title={t('createMode.repoPicker.removeRepository', {
                        repository: repoDisplayName,
                      })}
                      className={cn(repoRowButtonClassName, 'hover:text-error')}
                    >
                      <XIcon
                        className="size-icon-xs"
                        weight="bold"
                        aria-hidden="true"
                      />
                    </button>
                  </div>
                );
              })}
            </div>
          </div>
        )}

        <div className="agentos-create-workspace__repo-actions mt-base flex flex-wrap items-center gap-half">
          <button
            type="button"
            onClick={handleChooseRepo}
            disabled={isBusy}
            className={cn(
              recentInlineControlButtonClassName,
              repos.length > 0
                ? 'text-normal hover:text-high'
                : 'text-brand hover:text-brand-hover'
            )}
          >
            {pendingAction === 'choose' ? (
              <SpinnerIcon
                className="size-icon-xs animate-spin"
                aria-hidden="true"
              />
            ) : (
              <ClockCounterClockwiseIcon
                className="size-icon-xs"
                weight="bold"
                aria-hidden="true"
              />
            )}
            <span>{t('createMode.repoPicker.actions.recent')}</span>
          </button>
          <button
            type="button"
            onClick={handleBrowseRepo}
            disabled={isBusy}
            className={inlineControlButtonClassName}
          >
            {pendingAction === 'browse' ? (
              <SpinnerIcon
                className="size-icon-xs animate-spin"
                aria-hidden="true"
              />
            ) : (
              <MagnifyingGlassIcon
                className="size-icon-xs"
                weight="bold"
                aria-hidden="true"
              />
            )}
            <span>{t('createMode.repoPicker.actions.browse')}</span>
          </button>
          <button
            type="button"
            onClick={handleCreateRepo}
            disabled={isBusy}
            className={inlineControlButtonClassName}
          >
            {pendingAction === 'create' ? (
              <SpinnerIcon
                className="size-icon-xs animate-spin"
                aria-hidden="true"
              />
            ) : (
              <PlusIcon
                className="size-icon-xs"
                weight="bold"
                aria-hidden="true"
              />
            )}
            <span>{t('createMode.repoPicker.actions.create')}</span>
          </button>

          <div className="ml-auto">
            <PrimaryButton
              variant="default"
              value={t('createMode.repoPicker.continue')}
              onClick={onContinueToPrompt}
              disabled={isBusy || repos.length === 0}
              className="agentos-create-workspace__continue"
            />
          </div>
        </div>
      </div>
      {showSetupHint && (
        <div className="agentos-create-workspace__setup-hint mx-plusfifty mt-half flex items-start gap-half rounded-sm border border-brand/20 bg-brand/5 px-base py-base">
          <div className="flex-1">
            <p className="text-sm font-medium text-normal flex items-center gap-1.5">
              <SparkleIcon className="size-4 text-brand" weight="fill" />
              {t('createMode.repoPicker.setupHintTitle', 'Scripts de arranque y limpieza')}
            </p>
            <p className="mt-quarter text-sm text-low">
              {t(
                'createMode.repoPicker.setupHint',
                'Puedes auto-detectar los scripts según el stack del proyecto o configurarlos manualmente.'
              )}
            </p>
            <div className="mt-2 flex items-center gap-3">
              <button
                type="button"
                className="inline-flex items-center gap-1 rounded-sm bg-brand/15 px-2.5 py-1 text-xs font-semibold text-brand hover:bg-brand/25 transition-colors cursor-pointer"
                onClick={() => {
                  const unconfiguredRepo = repos.find(
                    (repo) => !repo.setup_script
                  );
                  if (unconfiguredRepo) {
                    void AutoDetectScriptsDialog.show({
                      repoId: unconfiguredRepo.id,
                      repoName: unconfiguredRepo.display_name || unconfiguredRepo.name,
                    });
                  }
                }}
              >
                <SparkleIcon className="size-3.5" weight="fill" />
                Auto-detectar scripts
              </button>
              <button
                type="button"
                className="cursor-pointer text-xs font-medium text-low underline hover:text-normal"
                onClick={() => {
                  const unconfiguredRepo = repos.find(
                    (repo) => !repo.setup_script
                  );
                  SettingsDialog.show({
                    initialSection: 'repos',
                    initialState: { repoId: unconfiguredRepo?.id },
                  });
                }}
              >
                {t('createMode.repoPicker.setupHintLink', 'Configurar en Ajustes')}
              </button>
            </div>
          </div>
          <button
            type="button"
            onClick={() => setSetupHintDismissed(true)}
            className="shrink-0 rounded-sm text-low hover:text-normal focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand cursor-pointer"
            aria-label={t('createMode.repoPicker.setupHintDismiss')}
          >
            <XIcon className="size-icon-2xs" weight="bold" aria-hidden="true" />
          </button>
        </div>
      )}
      {pickerError && (
        <div className="agentos-create-workspace__picker-error mt-half rounded-sm border border-error/30 bg-error/10 px-base py-half">
          <p className="text-xs text-error">{pickerError}</p>
        </div>
      )}
    </div>
  );
}
