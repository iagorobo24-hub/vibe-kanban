import { useMemo, useCallback } from 'react';
import { useParams } from '@tanstack/react-router';
import {
  PlusIcon,
  ArrowBendUpRightIcon,
  ProhibitIcon,
  ArrowsLeftRightIcon,
  CopyIcon,
} from '@phosphor-icons/react';
import { useProjectContext } from '@/shared/hooks/useProjectContext';
import { useActions } from '@/shared/hooks/useActions';
import { useAppNavigation } from '@/shared/hooks/useAppNavigation';
import { resolveRelationshipsForIssue } from '@/shared/lib/resolveRelationships';
import { IssueRelationshipsSection } from '@vibe/ui/components/IssueRelationshipsSection';
import {
  DropdownMenu,
  DropdownMenuTrigger,
  DropdownMenuContent,
  DropdownMenuItem,
} from '@vibe/ui/components/Dropdown';

interface IssueRelationshipsSectionContainerProps {
  issueId: string;
}

export function IssueRelationshipsSectionContainer({
  issueId,
}: IssueRelationshipsSectionContainerProps) {
  const { projectId } = useParams({ strict: false });
  const appNavigation = useAppNavigation();
  const { openRelationshipSelection } = useActions();

  const {
    getRelationshipsForIssue,
    removeIssueRelationship,
    issuesById,
    isLoading,
  } = useProjectContext();

  const relationships = useMemo(
    () =>
      resolveRelationshipsForIssue(
        issueId,
        getRelationshipsForIssue(issueId),
        issuesById
      ),
    [issueId, getRelationshipsForIssue, issuesById]
  );

  const handleRelationshipClick = useCallback(
    (relatedIssueId: string) => {
      if (!projectId) {
        return;
      }

      appNavigation.goToProjectIssue(projectId, relatedIssueId);
    },
    [projectId, appNavigation]
  );

  const handleRemoveRelationship = useCallback(
    (relationshipId: string) => {
      removeIssueRelationship(relationshipId);
    },
    [removeIssueRelationship]
  );

  const handleSelectType = useCallback(
    (
      relationshipType: 'blocking' | 'related' | 'has_duplicate',
      direction: 'forward' | 'reverse'
    ) => {
      if (projectId) {
        openRelationshipSelection(
          projectId,
          issueId,
          relationshipType,
          direction
        );
      }
    },
    [projectId, issueId, openRelationshipSelection]
  );

  const headerExtra = (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button
          type="button"
          aria-label="Añadir relación"
          className="rounded-sm p-0.5 text-low hover:text-normal focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand"
          onClick={(e) => e.stopPropagation()}
        >
          <PlusIcon
            aria-hidden="true"
            className="size-icon-xs"
            weight="bold"
          />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        <DropdownMenuItem
          icon={ArrowBendUpRightIcon}
          onSelect={() => handleSelectType('blocking', 'forward')}
        >
          Blocks...
        </DropdownMenuItem>
        <DropdownMenuItem
          icon={ProhibitIcon}
          onSelect={() => handleSelectType('blocking', 'reverse')}
        >
          Blocked by...
        </DropdownMenuItem>
        <DropdownMenuItem
          icon={ArrowsLeftRightIcon}
          onSelect={() => handleSelectType('related', 'forward')}
        >
          Related to...
        </DropdownMenuItem>
        <DropdownMenuItem
          icon={CopyIcon}
          onSelect={() => handleSelectType('has_duplicate', 'forward')}
        >
          Duplicate of...
        </DropdownMenuItem>
      </DropdownMenuContent>
    </DropdownMenu>
  );

  return (
    <IssueRelationshipsSection
      relationships={relationships}
      onRelationshipClick={handleRelationshipClick}
      onRemoveRelationship={handleRemoveRelationship}
      isLoading={isLoading}
      headerExtra={headerExtra}
    />
  );
}
