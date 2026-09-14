import type { ReactNode } from 'react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import type { Icon } from '@phosphor-icons/react';
import { CaretDownIcon } from '@phosphor-icons/react';
import { cn } from '../lib/cn';

const STORAGE_KEY_PREFIX = 'vibe.ui.collapsible.';

function getInitialExpanded(
  persistKey: string | undefined,
  defaultExpanded: boolean
) {
  if (!persistKey || typeof window === 'undefined') return defaultExpanded;
  try {
    const stored = window.localStorage.getItem(
      `${STORAGE_KEY_PREFIX}${persistKey}`
    );
    if (stored == null) return defaultExpanded;
    return stored === 'true';
  } catch {
    return defaultExpanded;
  }
}

export type SectionAction = {
  icon: Icon;
  onClick: () => void;
  isActive?: boolean;
  label?: string;
};

interface CollapsibleSectionHeaderProps {
  persistKey?: string;
  title: string;
  defaultExpanded?: boolean;
  collapsible?: boolean;
  actions?: SectionAction[];
  headerExtra?: ReactNode;
  children?: ReactNode;
  className?: string;
}

export function CollapsibleSectionHeader({
  persistKey,
  title,
  defaultExpanded = true,
  collapsible = true,
  actions = [],
  headerExtra,
  children,
  className,
}: CollapsibleSectionHeaderProps) {
  const { t } = useTranslation('common');
  const [expanded, setExpanded] = useState(() =>
    getInitialExpanded(persistKey, defaultExpanded)
  );

  useEffect(() => {
    setExpanded(getInitialExpanded(persistKey, defaultExpanded));
  }, [persistKey, defaultExpanded]);

  useEffect(() => {
    if (!persistKey) return;
    try {
      window.localStorage.setItem(
        `${STORAGE_KEY_PREFIX}${persistKey}`,
        String(expanded)
      );
    } catch {
      // Ignore localStorage failures (private mode/quota/security errors).
    }
  }, [persistKey, expanded]);

  const isExpanded = collapsible ? expanded : true;

  const titleContent = (
    <span className="agentos-collapsible-section__title font-medium truncate text-normal">
      {title}
    </span>
  );

  const actionContent = (
    <div className="agentos-collapsible-section__actions flex items-center gap-half">
      {headerExtra}
      {actions.map((action, index) => {
        const ActionIcon = action.icon;
        const label = action.label ?? `${title} action`;
        return (
          <button
            type="button"
            key={index}
            aria-label={label}
            title={label}
            onClick={action.onClick}
            className={cn(
              'agentos-collapsible-section__action rounded-sm border-0 bg-transparent p-half hover:text-normal',
              'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand',
              action.isActive ? 'text-brand' : 'text-low'
            )}
          >
            <ActionIcon className="size-icon-xs" weight="bold" aria-hidden="true" />
          </button>
        );
      })}
    </div>
  );

  const toggleIcon = (
    <CaretDownIcon
      weight="fill"
      className={cn(
        'agentos-collapsible-section__toggle size-icon-xs text-low transition-transform',
        !expanded && '-rotate-90'
      )}
      aria-hidden="true"
    />
  );

  const headerContent = collapsible ? (
    <div className="agentos-collapsible-section__header flex items-center w-full px-base py-half">
      <button
        type="button"
        onClick={() => setExpanded((prev) => !prev)}
        aria-expanded={expanded}
        className="flex min-w-0 flex-1 items-center text-left cursor-pointer border-0 bg-transparent p-0 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand"
      >
        {titleContent}
      </button>
      {actionContent}
      <button
        type="button"
        onClick={() => setExpanded((prev) => !prev)}
        aria-label={
          expanded
            ? t('accessibility.collapse', { title })
            : t('accessibility.expand', { title })
        }
        aria-expanded={expanded}
        className="ml-half shrink-0 rounded-sm border-0 bg-transparent p-half text-low hover:text-normal focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand"
      >
        {toggleIcon}
      </button>
    </div>
  ) : (
    <div className="agentos-collapsible-section__header flex items-center w-full px-base py-half">
      <span className="min-w-0 flex-1">{titleContent}</span>
      {actionContent}
    </div>
  );

  return (
    <div
      className={cn(
        'agentos-collapsible-section flex flex-col h-full min-h-0',
        className
      )}
    >
      <div>{headerContent}</div>
      {isExpanded && children}
    </div>
  );
}
