import { useTranslation } from "@/i18n/useTranslation";

export default function NotFoundPage() {
  const { t } = useTranslation("common");

  return (
    <div className="agentos-remote-not-found flex h-full items-center justify-center px-base">
      <div className="agentos-remote-card w-full max-w-md rounded-sm border border-border bg-secondary p-double text-center">
        <h1 className="text-xl font-semibold text-high">404</h1>
        <p className="mt-base text-low">{t("remote.pageNotFound")}</p>
      </div>
    </div>
  );
}
