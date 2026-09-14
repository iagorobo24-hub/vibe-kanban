'use client';

import type { ReactNode } from 'react';

import { cn } from '../lib/cn';
import {
  ArrowBendUpRightIcon,
  ProhibitIcon,
  ArrowsLeftRightIcon,
  CopyIcon,
} from '@phosphor-icons/react';

export type RelationshipDisplayType =
  | 'blocks'
  | 'blocked_by'
  | 'related'
  | 'duplicate_of'
  | 'duplicated_by';

export interface RelationshipBadgeProps {
  displayType: RelationshipDisplayType;
  relatedIssueDisplayId: string;
  compact?: boolean;
  className?: string;
  onClick?: (e: React.MouseEvent) => void;
}

const RELATIONSHIP_ICONS = {
  blocks: ArrowBendUpRightIcon,
  blocked_by: ProhibitIcon,
  related: ArrowsLeftRightIcon,
  duplicate_of: CopyIcon,
  duplicated_by: CopyIcon,
} as const;

function getRelationshipLabel(displayType: RelationshipDisplayType): string {
  switch (displayType) {
    case 'blocks':
      return 'blocks';
    case 'blocked_by':
      return 'blocked by';
    case 'related':
      return 'related';
    case 'duplicate_of':
      return 'dup of';
    case 'duplicated_by':
      return 'dup';
  }
}

export function RelationshipBadge({
  displayType,
  relatedIssueDisplayId,
  compact,
  className,
  onClick,
}: RelationshipBadgeProps) {
  const Icon = RELATIONSHIP_ICONS[displayType];
  const label = getRelationshipLabel(displayType);
  const isBlocking = displayType === 'blocks' || displayType === 'blocked_by';
  const badgeClassName = cn(
    'inline-flex items-center gap-half',
    'h-5 px-half',
    'rounded-sm',
    'text-sm font-medium',
    'whitespace-nowrap',
    isBlocking ? 'bg-error/10 text-error' : 'bg-panel text-low',
    onClick && 'cursor-pointer hover:opacity-80',
    className
  );
  const badgeContent: ReactNode = (
    <>
      <Icon className="size-icon-xs" weight="bold" aria-hidden="true" />
      <span>
        {compact ? relatedIssueDisplayId : `${label} ${relatedIssueDisplayId}`}
      </span>
    </>
  );

  if (onClick) {
    return (
      <button
        type="button"
        className={cn(
          badgeClassName,
          'border-0 appearance-none focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand'
        )}
        onClick={onClick}
        aria-label={`${label} ${relatedIssueDisplayId}`}
      >
        {badgeContent}
      </button>
    );
  }

  return <span className={badgeClassName}>{badgeContent}</span>;
}
