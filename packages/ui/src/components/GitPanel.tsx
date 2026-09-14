import {
  ArrowClockwiseIcon,
  GitBranchIcon,
  SpinnerGapIcon,
  WarningCircleIcon,
} from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/cn";
import { RepoCard, type RepoAction } from "./RepoCard";
import { InputField } from "./InputField";
import { PrimaryButton } from "./PrimaryButton";

export type GitPanelStatus = "loading" | "ready" | "unavailable";

export interface RepoInfo {
  id: string;
  name: string;
  targetBranch: string;
  commitsAhead?: number;
  commitsBehind?: number;
  remoteCommitsAhead?: number;
  prNumber?: number;
  prUrl?: string;
  prStatus?: "open" | "merged" | "closed" | "unknown";
  showPushButton?: boolean;
  isPushPending?: boolean;
  isPushSuccess?: boolean;
  isPushError?: boolean;
  isTargetRemote?: boolean;
}

interface GitPanelProps {
  repos: RepoInfo[];
  repoSelectedActions?: Record<string, RepoAction>;
  workingBranchName: string;
  onWorkingBranchNameChange: (name: string) => void;
  onActionsClick?: (repoId: string, action: RepoAction) => void;
  onRepoActionChange?: (repoId: string, action: RepoAction) => void;
  onPushClick?: (repoId: string) => void;
  onMoreClick?: (repoId: string) => void;
  onAddRepo?: () => void;
  className?: string;
  gitStatus?: GitPanelStatus;
  onRetryGitStatus?: () => void;
}

export function GitPanel({
  repos,
  repoSelectedActions,
  workingBranchName,
  onWorkingBranchNameChange,
  onActionsClick,
  onRepoActionChange,
  onPushClick,
  onMoreClick,
  className,
  gitStatus = "ready",
  onRetryGitStatus,
}: GitPanelProps) {
  const { t } = useTranslation(["tasks", "common"]);

  return (
    <div
      className={cn(
        "agentos-git-panel flex flex-col flex-1 w-full bg-secondary text-low overflow-y-auto",
        className,
      )}
    >
      {gitStatus === "loading" && repos.length > 0 && (
        <div
          className="mx-base mt-base flex items-center gap-half rounded-sm border border-border bg-primary px-base py-half text-sm text-low"
          role="status"
          aria-live="polite"
        >
          <SpinnerGapIcon
            className="size-icon-sm animate-spin"
            aria-hidden="true"
          />
          <span>{t("gitPanel.status.loading")}</span>
        </div>
      )}
      {gitStatus === "unavailable" && repos.length > 0 && (
        <div
          className="mx-base mt-base flex flex-wrap items-center justify-between gap-base rounded-sm border border-warning bg-warning/10 px-base py-half text-sm text-warning"
          role="alert"
        >
          <div className="flex min-w-0 items-center gap-half">
            <WarningCircleIcon
              className="size-icon-sm shrink-0"
              weight="fill"
              aria-hidden="true"
            />
            <span>{t("gitPanel.status.unavailable")}</span>
          </div>
          {onRetryGitStatus && (
            <PrimaryButton
              variant="tertiary"
              actionIcon={ArrowClockwiseIcon}
              onClick={onRetryGitStatus}
              className="shrink-0"
            >
              {t("gitPanel.status.retry")}
            </PrimaryButton>
          )}
        </div>
      )}
      <div className="gap-base px-base">
        {repos.length === 0 && (
          <div className="flex flex-col items-center gap-half px-base py-double text-center text-low">
            <GitBranchIcon className="size-icon-lg" aria-hidden="true" />
            <p className="font-medium text-normal">
              {t("gitPanel.empty.title")}
            </p>
            <p className="max-w-xs text-sm">
              {t("gitPanel.empty.description")}
            </p>
          </div>
        )}
        {repos.map((repo) => (
          <RepoCard
            key={repo.id}
            repoId={repo.id}
            name={repo.name}
            targetBranch={repo.targetBranch}
            commitsAhead={repo.commitsAhead}
            commitsBehind={repo.commitsBehind}
            prNumber={repo.prNumber}
            prUrl={repo.prUrl}
            prStatus={repo.prStatus}
            showPushButton={repo.showPushButton}
            isPushPending={repo.isPushPending}
            isPushSuccess={repo.isPushSuccess}
            isPushError={repo.isPushError}
            isTargetRemote={repo.isTargetRemote}
            selectedAction={repoSelectedActions?.[repo.id] ?? "pull-request"}
            onSelectedActionChange={(action) =>
              onRepoActionChange?.(repo.id, action)
            }
            onChangeTarget={() => onActionsClick?.(repo.id, "change-target")}
            onRebase={() => onActionsClick?.(repo.id, "rebase")}
            onActionsClick={(action) => onActionsClick?.(repo.id, action)}
            onPushClick={() => onPushClick?.(repo.id)}
            onMoreClick={() => onMoreClick?.(repo.id)}
          />
        ))}
        <div className="agentos-git-panel__working-branch bg-primary flex flex-col gap-base w-full p-base rounded-sm my-base">
          <div className="flex gap-base items-center">
            <GitBranchIcon className="size-icon-md text-base" weight="fill" />
            <p className="font-medium truncate">
              {t("common:sections.workingBranch")}
            </p>
          </div>
          <InputField
            variant="editable"
            value={workingBranchName}
            onChange={onWorkingBranchNameChange}
            placeholder={t("gitPanel.advanced.placeholder")}
          />
        </div>
      </div>
    </div>
  );
}
