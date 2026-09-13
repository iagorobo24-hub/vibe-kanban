import type { ReactNode } from 'react';
import { PlusIcon, HashIcon, GitPullRequest } from '@phosphor-icons/react';
import { cn } from '../lib/cn';
import { PRESET_COLORS } from './ColorPicker';
import { PrBadge, type PrBadgeStatus } from './PrBadge';
import { TAG_COLORS } from './SearchableTagDropdown';

// Re-export for backwards compatibility.
export { PRESET_COLORS, TAG_COLORS };

export interface IssueTagBase {
  id: string;
  name: string;
  color: string;
}

export interface LinkedPullRequest {
  id: string;
  number: number;
  url: string;
  status: PrBadgeStatus;
}

export interface LinkedIssue {
  id: string;
  displayId: string;
  title: string;
}

export interface IssueTagsRowProps<TTag extends IssueTagBase = IssueTagBase> {
  selectedTagIds: string[];
  availableTags: TTag[];
  linkedPrs?: LinkedPullRequest[];
  linkedIssues?: LinkedIssue[];
  onTagsChange: (tagIds: string[]) => void;
  onCreateTag?: (data: { name: string; color: string }) => string;
  renderAddTagControl?: (
    props: IssueTagsRowAddTagControlProps<TTag>
  ) => ReactNode;
  onLinkPr?: () => void;
  disabled?: boolean;
  className?: string;
}

export interface IssueTagsRowAddTagControlProps<
  TTag extends IssueTagBase = IssueTagBase,
> {
  tags: TTag[];
  selectedTagIds: string[];
  onTagToggle: (tagId: string) => void;
  onCreateTag: (data: { name: string; color: string }) => string;
  disabled: boolean;
  trigger: ReactNode;
}

export function IssueTagsRow<TTag extends IssueTagBase>({
  selectedTagIds,
  availableTags,
  linkedPrs = [],
  linkedIssues = [],
  onTagsChange,
  onCreateTag,
  renderAddTagControl,
  onLinkPr,
  disabled,
  className,
}: IssueTagsRowProps<TTag>) {
  const selectedTags = availableTags.filter((tag) =>
    selectedTagIds.includes(tag.id)
  );

  const handleTagToggle = (tagId: string) => {
    if (selectedTagIds.includes(tagId)) {
      onTagsChange(selectedTagIds.filter((id) => id !== tagId));
    } else {
      onTagsChange([...selectedTagIds, tagId]);
    }
  };

  const handleCreateTag = (data: { name: string; color: string }): string => {
    return onCreateTag?.(data) ?? '';
  };

  const addTagTrigger = (
    <button
      type="button"
      className="flex size-6 items-center justify-center rounded-sm text-low transition-colors hover:bg-panel hover:text-normal focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand disabled:opacity-50"
      disabled={disabled}
      aria-label="Add tag"
    >
      <PlusIcon className="size-icon-xs" weight="bold" aria-hidden="true" />
    </button>
  );

  return (
    <div className={cn('flex items-center gap-half flex-wrap', className)}>
      {/* Selected Tags - clickable to remove on hover */}
      {selectedTags.map((tag) => (
        <button
          key={tag.id}
          type="button"
          onClick={() => handleTagToggle(tag.id)}
          disabled={disabled}
          aria-label={`Remove tag ${tag.name}`}
          className={cn(
            'inline-flex items-center justify-center',
            'h-6 gap-half px-base',
            'bg-panel rounded-sm',
            'text-sm text-low font-medium',
            'whitespace-nowrap',
            'transition-colors',
            !disabled &&
              'hover:bg-error/20 hover:text-error hover:line-through cursor-pointer',
            disabled && 'cursor-default',
            'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand'
          )}
        >
          <span
            className="w-2 h-2 rounded-full shrink-0"
            style={{ backgroundColor: `hsl(${tag.color})` }}
            aria-hidden="true"
          />
          {tag.name}
        </button>
      ))}

      {/* Linked PRs */}
      {linkedPrs.map((pr) => (
        <PrBadge
          key={pr.id}
          number={pr.number}
          url={pr.url}
          status={pr.status}
        />
      ))}

      {/* Link PR button */}
      {onLinkPr && (
        <button
          type="button"
          onClick={onLinkPr}
          disabled={disabled}
          className="flex size-6 items-center justify-center rounded-sm text-low transition-colors hover:bg-panel hover:text-normal focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand disabled:opacity-50"
          aria-label="Link pull request"
        >
          <GitPullRequest
            className="size-icon-xs"
            weight="bold"
            aria-hidden="true"
          />
        </button>
      )}

      {/* Linked Issues */}
      {linkedIssues.map((issue) => (
        <button
          key={issue.id}
          type="button"
          className="inline-flex items-center gap-half rounded-sm bg-panel px-base py-half text-sm text-low transition-colors hover:text-normal focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand"
          title={issue.title}
          aria-label={`${issue.displayId}: ${issue.title}`}
        >
          <HashIcon className="size-icon-xs" weight="bold" aria-hidden="true" />
          <span>{issue.displayId}</span>
        </button>
      ))}

      {/* Add Tag Dropdown */}
      {onCreateTag &&
        (renderAddTagControl?.({
          tags: availableTags,
          selectedTagIds,
          onTagToggle: handleTagToggle,
          onCreateTag: handleCreateTag,
          disabled: disabled ?? false,
          trigger: addTagTrigger,
        }) ??
          addTagTrigger)}
    </div>
  );
}
