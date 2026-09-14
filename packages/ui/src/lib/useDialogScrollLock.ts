import { useEffect } from "react";

/**
 * Prevents the page behind a modal or drawer from scrolling while preserving
 * whatever overflow value the host application had before the surface opened.
 */
export function useDialogScrollLock(open: boolean) {
  useEffect(() => {
    if (!open) return;

    const previousOverflow = document.body.style.overflow;
    document.body.style.overflow = "hidden";

    return () => {
      document.body.style.overflow = previousOverflow;
    };
  }, [open]);
}
