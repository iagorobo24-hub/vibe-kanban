import { useEffect, useState } from "react";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { redeemOAuth } from "@remote/shared/lib/api";
import { storeTokens } from "@remote/shared/lib/auth";
import { retrieveVerifier, clearVerifier } from "@remote/shared/lib/pkce";
import { RemoteStatusCard } from "@remote/shared/components/RemotePagePrimitives";

function getSafeNextPath(nextPath: string | undefined): string {
  if (!nextPath) {
    return "/";
  }

  if (!nextPath.startsWith("/") || nextPath.startsWith("//")) {
    return "/";
  }

  return nextPath;
}

export default function LoginCompletePage() {
  const navigate = useNavigate();
  const search = useSearch({ from: "/account_/complete" });
  const [error, setError] = useState<string | null>(null);

  const handoffId = search.handoff_id;
  const appCode = search.app_code;
  const oauthError = search.error;
  const nextPath = getSafeNextPath(search.next);

  useEffect(() => {
    const complete = async () => {
      if (oauthError) {
        setError(`OAuth error: ${oauthError}`);
        return;
      }

      if (!handoffId || !appCode) {
        return;
      }

      try {
        const verifier = retrieveVerifier();
        if (!verifier) {
          setError("OAuth session lost. Please try again.");
          return;
        }

        const { access_token, refresh_token } = await redeemOAuth(
          handoffId,
          appCode,
          verifier,
        );

        await storeTokens(access_token, refresh_token);
        clearVerifier();

        window.location.replace(nextPath);
      } catch (e) {
        setError(e instanceof Error ? e.message : "Failed to complete login");
        clearVerifier();
      }
    };

    void complete();
  }, [handoffId, appCode, oauthError, nextPath]);

  if (error) {
    return (
      <RemoteStatusCard title="Login failed" variant="error">
        <p className="text-sm text-normal mt-base">{error}</p>
        <button
          type="button"
          className="mt-double w-full rounded-sm bg-brand px-base py-half text-sm font-medium text-on-brand transition-colors hover:bg-brand-hover"
          onClick={() =>
            navigate({
              to: "/account",
              search: nextPath !== "/" ? { next: nextPath } : undefined,
              replace: true,
            })
          }
        >
          Try again
        </button>
      </RemoteStatusCard>
    );
  }

  return (
    <RemoteStatusCard title="Completing login...">
      <p className="text-sm text-low mt-base">Processing OAuth callback...</p>
    </RemoteStatusCard>
  );
}
