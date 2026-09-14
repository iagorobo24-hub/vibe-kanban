import { useEffect, useRef, type RefObject } from "react";

const FOCUSABLE_DIALOG_SELECTOR =
  'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [contenteditable="true"], [tabindex]:not([tabindex="-1"])';

/**
 * Gives a custom dialog the same focus lifecycle as a Radix dialog:
 * focus enters the first control, Tab stays inside, and focus returns to the
 * element that opened the dialog when it closes.
 */
export function useDialogFocusTrap<T extends HTMLElement>(
  dialogRef: RefObject<T>,
  open: boolean,
) {
  const previouslyFocusedElementRef = useRef<HTMLElement | null>(null);

  useEffect(() => {
    if (!open) return;

    previouslyFocusedElementRef.current =
      document.activeElement instanceof HTMLElement
        ? document.activeElement
        : null;

    const frame = window.requestAnimationFrame(() => {
      const dialog = dialogRef.current;
      if (!dialog) return;

      const firstFocusable = dialog.querySelector<HTMLElement>(
        FOCUSABLE_DIALOG_SELECTOR,
      );
      (firstFocusable ?? dialog).focus();
    });

    return () => {
      window.cancelAnimationFrame(frame);
      const previouslyFocusedElement = previouslyFocusedElementRef.current;
      if (previouslyFocusedElement?.isConnected) {
        previouslyFocusedElement.focus();
      }
      previouslyFocusedElementRef.current = null;
    };
  }, [dialogRef, open]);

  useEffect(() => {
    if (!open) return;

    const dialog = dialogRef.current;
    if (!dialog) return;

    const handleTabKey = (event: KeyboardEvent) => {
      if (event.key !== "Tab") return;

      const focusableElements = Array.from(
        dialog.querySelectorAll<HTMLElement>(FOCUSABLE_DIALOG_SELECTOR),
      );

      if (focusableElements.length === 0) {
        event.preventDefault();
        dialog.focus();
        return;
      }

      const firstFocusable = focusableElements[0];
      const lastFocusable = focusableElements[focusableElements.length - 1];
      const activeElement = document.activeElement;

      if (event.shiftKey && activeElement === firstFocusable) {
        event.preventDefault();
        lastFocusable.focus();
      } else if (!event.shiftKey && activeElement === lastFocusable) {
        event.preventDefault();
        firstFocusable.focus();
      }
    };

    // Listen at document level so a programmatic focus change outside the
    // dialog cannot make the next Tab escape the modal surface.
    document.addEventListener("keydown", handleTabKey, true);
    return () => document.removeEventListener("keydown", handleTabKey, true);
  }, [dialogRef, open]);
}
