'use client';

import { useTranslation } from 'react-i18next';
import { XIcon } from '@phosphor-icons/react';
import { CollapsibleSectionHeader } from './CollapsibleSectionHeader';
import {
  RelationshipBadge,
  type RelationshipDisplayType,
} from './RelationshipBadge';

export interface IssueRelationshipsSectionRelationship {
  relationshipId: string;
  relatedIssueId: string;
  relatedIssueDisplayId: string;
  displayType: RelationshipDisplayType;
}

export interface IssueRelationshipsSectionProps {
  relationships: IssueRelationshipsSectionRelationship[];
  onRelationshipClick: (relatedIssueId: string) => void;
  onRemoveRelationship?: (relationshipId: string) => void;
  isLoading?: boolean;
  headerExtra?: React.ReactNode;
}

export function IssueRelationshipsSection({
  relationships,
  onRelationshipClick,
  onRemoveRelationship,
  isLoading,
  headerExtra,
}: IssueRelationshipsSectionProps) {
  const { t } = useTranslation('common');

  return (
    <CollapsibleSectionHeader
      title={t('kanban.relationships', 'Relationships')}
      persistKey="kanban-issue-relationships"
      defaultExpanded={true}
      headerExtra={headerExtra}
      className="agentos-issue-relationships-section"
    >
      <div className="agentos-issue-section p-base flex flex-col gap-half border-t">
        {isLoading ? (
          <p className="text-low py-half">{t('states.loading')}</p>
        ) : relationships.length === 0 ? (
          <p className="text-low py-half">
            {t('kanban.noRelationships', 'No relationships')}
          </p>
        ) : (
          relationships.map((rel) => (
            <div
              key={rel.relationshipId}
              className="agentos-issue-relationship-row flex items-center justify-between group"
            >
              <RelationshipBadge
                displayType={rel.displayType}
                relatedIssueDisplayId={rel.relatedIssueDisplayId}
                onClick={(e) => {
                  e.stopPropagation();
                  onRelationshipClick(rel.relatedIssueId);
                }}
              />
              {onRemoveRelationship && (
                <button
                  type="button"
                  onClick={(e) => {
                    e.stopPropagation();
                    onRemoveRelationship(rel.relationshipId);
                  }}
                  className="agentos-issue-relationship-row__remove rounded-sm p-half text-low opacity-0 transition-colors hover:bg-error/10 hover:text-error focus-visible:opacity-100 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand group-hover:opacity-100"
                  aria-label={t('accessibility.removeRelationship')}
                >
                  <XIcon
                    className="size-icon-2xs"
                    weight="bold"
                    aria-hidden="true"
                  />
                </button>
              )}
            </div>
          ))
        )}
      </div>
    </CollapsibleSectionHeader>
  );
}
