import type { KeyboardEvent, MouseEvent, ReactNode } from 'react';
import { useEffect, useState } from 'react';
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

  const handleActionClick = (
    e: MouseEvent<HTMLSpanElement>,
    onClick: () => void
  ) => {
    e.stopPropagation();
    onClick();
  };

  const handleActionKeyDown = (
    e: KeyboardEvent<HTMLSpanElement>,
    onClick: () => void
  ) => {
    if (e.key !== 'Enter' && e.key !== ' ') return;
    e.preventDefault();
    e.stopPropagation();
    onClick();
  };

  const isExpanded = collapsible ? expanded : true;

  const headerContent = (
    <>
      <span className="agentos-collapsible-section__title font-medium truncate text-normal">
        {title}
      </span>
      <div className="agentos-collapsible-section__actions flex items-center gap-half">
        {headerExtra}
        {actions.map((action, index) => {
          const ActionIcon = action.icon;
          return (
            <span
              key={index}
              role="button"
              tabIndex={0}
              onClick={(e) => handleActionClick(e, action.onClick)}
              onKeyDown={(e) => handleActionKeyDown(e, action.onClick)}
              className={cn(
                'agentos-collapsible-section__action hover:text-normal',
                action.isActive ? 'text-brand' : 'text-low'
              )}
            >
              <ActionIcon className="size-icon-xs" weight="bold" />
            </span>
          );
        })}
        {collapsible && (
          <CaretDownIcon
            weight="fill"
            className={cn(
              'agentos-collapsible-section__toggle size-icon-xs text-low transition-transform',
              !expanded && '-rotate-90'
            )}
          />
        )}
      </div>
    </>
  );

  return (
    <div
      className={cn(
        'agentos-collapsible-section flex flex-col h-full min-h-0',
        className
      )}
    >
      <div className="">
        {collapsible ? (
          <button
            type="button"
            onClick={() => setExpanded((prev) => !prev)}
            className={cn(
              'agentos-collapsible-section__header flex items-center justify-between w-full px-base py-half cursor-pointer'
            )}
          >
            {headerContent}
          </button>
        ) : (
          <div
            className={cn(
              'agentos-collapsible-section__header flex items-center justify-between w-full px-base py-half'
            )}
          >
            {headerContent}
          </div>
        )}
      </div>
      {isExpanded && children}
    </div>
  );
}
