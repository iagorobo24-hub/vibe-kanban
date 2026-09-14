import { ComponentType } from "react";
import { useTranslation } from "react-i18next";
import {
  CaretDownIcon,
  UserIcon,
  ListChecksIcon,
  GearIcon,
  IconProps,
} from "@phosphor-icons/react";
import { cn } from "../lib/cn";

type Variant = "user" | "plan" | "plan_denied" | "system";

export interface ChatEntryStatusLike {
  status: string;
}

interface VariantConfig {
  icon: ComponentType<IconProps>;
  border: string;
  headerBg: string;
  bg: string;
}

const variantConfig: Record<Variant, VariantConfig> = {
  user: {
    icon: UserIcon,
    border: "border-border",
    headerBg: "",
    bg: "",
  },
  plan: {
    icon: ListChecksIcon,
    border: "border-brand",
    headerBg: "bg-brand/20",
    bg: "bg-brand/10",
  },
  plan_denied: {
    icon: ListChecksIcon,
    border: "border-error",
    headerBg: "bg-error/20",
    bg: "bg-error/10",
  },
  system: {
    icon: GearIcon,
    border: "border-border",
    headerBg: "bg-gray-50 dark:bg-gray-900/30",
    bg: "",
  },
};

interface ChatEntryContainerProps {
  variant: Variant;
  title?: React.ReactNode;
  headerRight?: React.ReactNode;
  expanded?: boolean;
  onToggle?: () => void;
  children?: React.ReactNode;
  actions?: React.ReactNode;
  className?: string;
  status?: ChatEntryStatusLike;
  isGreyed?: boolean;
}

export function ChatEntryContainer({
  variant,
  title,
  headerRight,
  expanded = false,
  onToggle,
  children,
  actions,
  className,
  status,
  isGreyed,
}: ChatEntryContainerProps) {
  const { t } = useTranslation("common");
  // Special case for plan denied
  const config =
    variant === "plan" && status?.status === "denied"
      ? variantConfig.plan_denied
      : variantConfig[variant];
  const Icon = config.icon;
  const headerClassName = cn(
    "flex items-center px-double py-base gap-base rounded-sm overflow-hidden",
    config.headerBg,
  );

  const titleContent = title && (
    <span className="flex-1 min-w-0 text-sm text-normal truncate">{title}</span>
  );

  const toggleButtonClassName =
    "flex min-w-0 flex-1 items-center gap-base border-0 bg-transparent p-0 text-left focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand";

  return (
    <div
      className={cn(
        "rounded-sm w-full",
        config.border && "border",
        config.border,
        config.bg,
        isGreyed && "opacity-50 pointer-events-none",
        className,
      )}
    >
      {/* Header */}
      {onToggle ? (
        <div className={cn(headerClassName, "cursor-pointer")}>
          <button
            type="button"
            className={toggleButtonClassName}
            onClick={onToggle}
            aria-expanded={expanded}
          >
            <Icon
              className="size-icon-xs shrink-0 text-low"
              aria-hidden="true"
            />
            {titleContent}
          </button>
          {headerRight}
          <button
            type="button"
            className="shrink-0 rounded-sm border-0 bg-transparent p-half text-low hover:text-normal focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand"
            onClick={onToggle}
            aria-label={
              expanded
                ? t("accessibility.collapseMessage")
                : t("accessibility.expandMessage")
            }
            aria-expanded={expanded}
          >
            <CaretDownIcon
              className={cn(
                "size-icon-xs transition-transform",
                !expanded && "-rotate-90",
              )}
              aria-hidden="true"
            />
          </button>
        </div>
      ) : (
        <div className={headerClassName}>
          <Icon className="size-icon-xs shrink-0 text-low" aria-hidden="true" />
          {titleContent}
          {headerRight}
        </div>
      )}

      {/* Content - shown when expanded */}
      {expanded && children && <div className="p-double">{children}</div>}

      {/* Actions footer - optional */}
      {actions && (
        <div className="bg-brand/20 backdrop-blur-sm flex items-center gap-base px-double py-base border-t sticky bottom-0 rounded-md">
          {actions}
        </div>
      )}
    </div>
  );
}
