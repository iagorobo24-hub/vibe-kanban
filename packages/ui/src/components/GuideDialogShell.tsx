import { useRef, useState, type ReactNode } from "react";
import { CaretLeftIcon, XIcon } from "@phosphor-icons/react";
import { cn } from "../lib/cn";
import { useDialogFocusTrap } from "../lib/useDialogFocusTrap";

export interface GuideDialogTopic {
  id: string;
  title: string;
  content: ReactNode;
  imageSrc?: string;
  imageAlt?: string;
}

interface GuideDialogShellProps {
  topics: GuideDialogTopic[];
  closeLabel: string;
  onClose: () => void;
  className?: string;
}

export function GuideDialogShell({
  topics,
  closeLabel,
  onClose,
  className,
}: GuideDialogShellProps) {
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [mobileShowContent, setMobileShowContent] = useState(false);
  const dialogRef = useRef<HTMLDivElement | null>(null);

  useDialogFocusTrap(dialogRef, topics.length > 0);

  if (topics.length === 0) {
    return null;
  }

  const selectedTopic = topics[selectedIndex] ?? topics[0];

  return (
    <>
      <div
        data-tauri-drag-region
        className="agentos-dialog-overlay fixed inset-0 z-[9998] bg-black/50 animate-in fade-in-0 duration-200"
        onClick={onClose}
        aria-hidden="true"
      />
      {/* Dialog wrapper - handles positioning */}
      <div
        className={cn(
          "fixed z-[9999]",
          // Mobile: full screen
          "inset-0",
          // Desktop: centered with fixed size
          "md:inset-auto md:left-1/2 md:top-1/2 md:-translate-x-1/2 md:-translate-y-1/2",
        )}
      >
        <div
          ref={dialogRef}
          className={cn(
            "agentos-guide-dialog h-full w-full flex overflow-hidden",
            "bg-panel/95 backdrop-blur-sm shadow-lg",
            "animate-in fade-in-0 slide-in-from-bottom-4 duration-200",
            // Mobile: full screen, no rounded corners
            "rounded-none border-0",
            // Desktop: fixed size with rounded corners
            "md:w-[800px] md:h-[600px] md:rounded-sm md:border md:border-border/50",
            className,
          )}
          role="dialog"
          aria-modal="true"
          aria-labelledby="agentos-guide-dialog-title"
        >
          {/* Sidebar - hidden on mobile when showing content */}
          <div
            className={cn(
              "agentos-guide-dialog__nav bg-secondary/80 border-r border-border/50 flex flex-col",
              // Mobile: full width, hidden when showing content
              "w-full",
              mobileShowContent && "hidden",
              // Desktop: fixed width sidebar, always visible
              "md:w-52 md:block",
            )}
          >
            {/* Header with mobile close button */}
            <div className="p-3 flex items-center justify-between md:hidden">
              <span className="text-sm font-medium text-high">Topics</span>
              <button
                type="button"
                onClick={onClose}
                aria-label={closeLabel}
                className="p-1 rounded-sm hover:bg-secondary text-low hover:text-normal"
              >
                <XIcon className="h-4 w-4" weight="bold" aria-hidden="true" />
              </button>
            </div>
            <nav
              className="flex-1 p-3 flex flex-col gap-1 overflow-y-auto md:pt-3"
              aria-label="Guide topics"
            >
              {topics.map((topic, idx) => (
                <button
                  key={topic.id}
                  type="button"
                  onClick={() => {
                    setSelectedIndex(idx);
                    setMobileShowContent(true);
                  }}
                  aria-pressed={idx === selectedIndex}
                  className={cn(
                    "text-left px-3 py-2 rounded-sm text-sm transition-colors",
                    idx === selectedIndex
                      ? "bg-brand/10 text-brand font-medium"
                      : "text-normal hover:bg-primary/10",
                  )}
                >
                  {topic.title}
                </button>
              ))}
            </nav>
          </div>
          {/* Content - hidden on mobile when showing nav */}
          <div
            className={cn(
              "agentos-guide-dialog__body flex-1 flex flex-col relative overflow-y-auto",
              // Mobile: full width, hidden when showing nav
              !mobileShowContent && "hidden",
              // Desktop: always visible
              "md:flex",
            )}
          >
            {/* Mobile header with back button */}
            <div className="flex items-center gap-2 p-3 border-b border-border/50 md:hidden">
              <button
                type="button"
                onClick={() => setMobileShowContent(false)}
                aria-label="Back"
                className="p-1 rounded-sm hover:bg-secondary text-low hover:text-normal"
              >
                <CaretLeftIcon
                  className="h-4 w-4"
                  weight="bold"
                  aria-hidden="true"
                />
              </button>
              <span className="text-sm font-medium text-high">Back</span>
              <button
                type="button"
                onClick={onClose}
                aria-label={closeLabel}
                className="ml-auto p-1 rounded-sm hover:bg-secondary text-low hover:text-normal"
              >
                <XIcon className="h-4 w-4" weight="bold" aria-hidden="true" />
              </button>
            </div>
            {/* Desktop close button */}
            <button
              type="button"
              onClick={onClose}
              aria-label={closeLabel}
              className="absolute right-4 top-4 rounded-sm opacity-70 ring-offset-panel transition-opacity hover:opacity-100 focus:outline-none focus:ring-2 focus:ring-brand focus:ring-offset-2 hidden md:block"
            >
              <XIcon className="h-4 w-4 text-normal" aria-hidden="true" />
            </button>
            <div className="p-6 pt-4 md:pt-6 flex-1">
              <h2
                id="agentos-guide-dialog-title"
                className="text-xl font-semibold text-high mb-4 pr-8"
              >
                {selectedTopic.title}
              </h2>
              {selectedTopic.imageSrc && (
                <img
                  src={selectedTopic.imageSrc}
                  alt={selectedTopic.imageAlt ?? selectedTopic.title}
                  className="w-full rounded-sm border border-border/30 mb-4"
                />
              )}
              <div className="text-normal text-sm leading-relaxed space-y-3">
                {typeof selectedTopic.content === "string" ? (
                  <p>{selectedTopic.content}</p>
                ) : (
                  selectedTopic.content
                )}
              </div>
            </div>
          </div>
        </div>
      </div>
    </>
  );
}
