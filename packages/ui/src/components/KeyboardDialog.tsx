import * as React from "react";
import { X } from "lucide-react";
import { useHotkeys, useHotkeysContext } from "react-hotkeys-hook";
import { createPortal } from "react-dom";

import { cn } from "../lib/cn";
import { useDialogFocusTrap } from "../lib/useDialogFocusTrap";

const DIALOG_SCOPE = "dialog";
const KANBAN_SCOPE = "kanban";
const PROJECTS_SCOPE = "projects";
const DialogTitleIdContext = React.createContext<string | null>(null);

function assignRef<T>(ref: React.ForwardedRef<T>, value: T | null) {
  if (typeof ref === "function") {
    ref(value);
    return;
  }
  if (ref) {
    ref.current = value;
  }
}

const Dialog = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement> & {
    open?: boolean;
    onOpenChange?: (open: boolean) => void;
    uncloseable?: boolean;
  }
>(({ className, open, onOpenChange, children, uncloseable, ...props }, ref) => {
  const { enableScope, disableScope } = useHotkeysContext();
  const dialogRef = React.useRef<HTMLDivElement | null>(null);
  const dialogTitleId = React.useId();

  const setDialogRef = React.useCallback(
    (node: HTMLDivElement | null) => {
      dialogRef.current = node;
      assignRef(ref, node);
    },
    [ref],
  );

  useDialogFocusTrap(dialogRef, !!open);

  // Manage dialog scope when open/closed
  React.useEffect(() => {
    if (open) {
      enableScope(DIALOG_SCOPE);
      disableScope(KANBAN_SCOPE);
      disableScope(PROJECTS_SCOPE);
    } else {
      disableScope(DIALOG_SCOPE);
      enableScope(KANBAN_SCOPE);
      enableScope(PROJECTS_SCOPE);
    }
    return () => {
      disableScope(DIALOG_SCOPE);
      enableScope(KANBAN_SCOPE);
      enableScope(PROJECTS_SCOPE);
    };
  }, [open, enableScope, disableScope]);

  useHotkeys(
    "esc",
    (e) => {
      if (!open) return;
      if (uncloseable) return;

      const activeElement = document.activeElement as HTMLElement;
      if (
        activeElement &&
        (activeElement.tagName === "INPUT" ||
          activeElement.tagName === "TEXTAREA" ||
          activeElement.isContentEditable)
      ) {
        activeElement.blur();
        e?.preventDefault();
        return;
      }

      onOpenChange?.(false);
    },
    {
      enabled: !!open,
      scopes: [DIALOG_SCOPE],
      preventDefault: true,
    },
    [open, uncloseable, onOpenChange],
  );

  useHotkeys(
    "enter",
    (e) => {
      if (!open) return;

      const activeElement = document.activeElement as HTMLElement;
      if (activeElement?.tagName === "TEXTAREA") {
        return;
      }

      const container = dialogRef.current;
      if (!container) {
        return;
      }

      const submitButton = container.querySelector(
        'button[type="submit"]',
      ) as HTMLButtonElement | null;
      if (submitButton && !submitButton.disabled) {
        e?.preventDefault();
        submitButton.click();
        return;
      }

      const buttons = Array.from(
        container.querySelectorAll("button"),
      ) as HTMLButtonElement[];
      const primaryButton = buttons.find(
        (btn) =>
          !btn.disabled &&
          !btn.textContent?.toLowerCase().includes("cancel") &&
          !btn.textContent?.toLowerCase().includes("close") &&
          btn.type !== "button",
      );

      if (primaryButton) {
        e?.preventDefault();
        primaryButton.click();
      }
    },
    {
      enabled: !!open,
      scopes: [DIALOG_SCOPE],
    },
    [open],
  );

  if (!open) return null;

  return createPortal(
    <div className="agentos-dialog agentos-dialog-viewport fixed inset-0 z-[10000] flex items-start justify-center p-4 overflow-y-auto overscroll-contain">
      <div
        data-tauri-drag-region
        className="agentos-dialog-overlay fixed inset-0 bg-black/50"
        onClick={() => (uncloseable ? {} : onOpenChange?.(false))}
      />
      <div
        ref={setDialogRef}
        className={cn(
          "agentos-dialog-content relative z-[10000] flex flex-col w-full max-w-xl gap-4 bg-primary p-6 shadow-lg duration-200 sm:rounded-lg my-8",
          className,
        )}
        {...props}
        role="dialog"
        aria-modal="true"
        aria-labelledby={dialogTitleId}
        tabIndex={-1}
      >
        {!uncloseable && (
          <button
            type="button"
            className="agentos-dialog-close absolute right-4 top-4 rounded-sm opacity-70 ring-offset-background transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2 z-10"
            onClick={() => onOpenChange?.(false)}
          >
            <X className="h-4 w-4" aria-hidden="true" />
            <span className="sr-only">Close</span>
          </button>
        )}
        <DialogTitleIdContext.Provider value={dialogTitleId}>
          {children}
        </DialogTitleIdContext.Provider>
      </div>
    </div>,
    document.body,
  );
});
Dialog.displayName = "Dialog";

const DialogHeader = ({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div
    className={cn(
      "agentos-dialog-header flex flex-col space-y-1.5 text-center sm:text-left",
      className,
    )}
    {...props}
  />
);
DialogHeader.displayName = "DialogHeader";

const DialogTitle = React.forwardRef<
  HTMLParagraphElement,
  React.HTMLAttributes<HTMLHeadingElement>
>(({ className, id, ...props }, ref) => {
  const contextId = React.useContext(DialogTitleIdContext);

  return (
    <h3
      ref={ref}
      id={id ?? contextId ?? undefined}
      className={cn(
        "agentos-dialog-title text-lg font-semibold leading-none tracking-tight",
        className,
      )}
      {...props}
    />
  );
});
DialogTitle.displayName = "DialogTitle";

const DialogDescription = React.forwardRef<
  HTMLParagraphElement,
  React.HTMLAttributes<HTMLParagraphElement>
>(({ className, ...props }, ref) => (
  <p
    ref={ref}
    className={cn(
      "agentos-dialog-description text-sm text-muted-foreground",
      className,
    )}
    {...props}
  />
));
DialogDescription.displayName = "DialogDescription";

const DialogContent = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div
    ref={ref}
    className={cn(
      "agentos-dialog-content__body flex flex-col gap-4",
      className,
    )}
    {...props}
  />
));
DialogContent.displayName = "DialogContent";

const DialogFooter = ({
  className,
  ...props
}: React.HTMLAttributes<HTMLDivElement>) => (
  <div
    className={cn(
      "agentos-dialog-footer flex flex-col-reverse gap-2 sm:flex-row sm:justify-end sm:space-x-2",
      className,
    )}
    {...props}
  />
);
DialogFooter.displayName = "DialogFooter";

export {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
};
