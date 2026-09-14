import { ArrowLeftIcon, ArrowRightIcon } from "@phosphor-icons/react";
import { useTranslation } from "react-i18next";
import { IconButtonGroup, IconButtonGroupItem } from "./IconButtonGroup";

export interface PreviewNavigationState {
  canGoBack: boolean;
  canGoForward: boolean;
}

interface PreviewNavigationProps {
  navigation: PreviewNavigationState | null;
  onBack: () => void;
  onForward: () => void;
  disabled?: boolean;
  className?: string;
}

export function PreviewNavigation({
  navigation,
  onBack,
  onForward,
  disabled = false,
  className,
}: PreviewNavigationProps) {
  const { t } = useTranslation(["tasks"]);

  return (
    <IconButtonGroup className={className}>
      <IconButtonGroupItem
        icon={ArrowLeftIcon}
        onClick={onBack}
        disabled={!navigation?.canGoBack || disabled}
        aria-label={t("preview.toolbar.goBack")}
        title={t("preview.toolbar.goBack")}
      />
      <IconButtonGroupItem
        icon={ArrowRightIcon}
        onClick={onForward}
        disabled={!navigation?.canGoForward || disabled}
        aria-label={t("preview.toolbar.goForward")}
        title={t("preview.toolbar.goForward")}
      />
    </IconButtonGroup>
  );
}
