import { type ReactNode } from 'react';
import { createPortal } from 'react-dom';

import { cn } from '../lib/cn';

interface MobileDrawerProps {
  open: boolean;
  onClose: () => void;
  children: ReactNode;
  ariaLabel?: string;
}

export function MobileDrawer({
  open,
  onClose,
  children,
  ariaLabel = 'AgentOS navigation',
}: MobileDrawerProps) {
  return createPortal(
    <>
      {/* Backdrop overlay */}
      <div
        data-tauri-drag-region
        className={cn(
          'agentos-mobile-drawer__backdrop',
          'fixed inset-0 bg-black/50 z-[100]',
          'transition-opacity duration-200 ease-out',
          open ? 'opacity-100' : 'opacity-0 pointer-events-none'
        )}
        onClick={onClose}
        aria-hidden="true"
      />
      {/* Drawer panel */}
      <div
        role="dialog"
        aria-modal={open || undefined}
        aria-hidden={!open}
        aria-label={ariaLabel}
        tabIndex={-1}
        className={cn(
          'agentos-theme agentos-mobile-drawer',
          'fixed left-0 top-0 h-full w-[280px] bg-primary z-[101]',
          'pb-[env(safe-area-inset-bottom)]',
          'transition-transform duration-200 ease-out',
          open ? 'translate-x-0' : '-translate-x-full pointer-events-none'
        )}
      >
        {children}
      </div>
    </>,
    document.body
  );
}
