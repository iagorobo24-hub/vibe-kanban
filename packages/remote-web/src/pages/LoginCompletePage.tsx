import { useEffect, useState } from 'react';
import { useTranslation } from '@/i18n/useTranslation';
import { useNavigate, useSearch } from '@tanstack/react-router';
import { redeemOAuth } from '@remote/shared/lib/api';
import { storeTokens } from '@remote/shared/lib/auth';
import { retrieveVerifier, clearVerifier } from '@remote/shared/lib/pkce';
import { RemoteStatusCard } from '@remote/shared/components/RemotePagePrimitives';

function getSafeNextPath(nextPath: string | undefined): string {
  if (!nextPath) {
    return '/';
  }

  if (!nextPath.startsWith('/') || nextPath.startsWith('//')) {
    return '/';
  }

  return nextPath;
}

export default function LoginCompletePage() {
  const navigate = useNavigate();
  const { t } = useTranslation('common');
  const search = useSearch({ from: '/account_/complete' });
  const [error, setError] = useState<string | null>(null);

  const handoffId = search.handoff_id;
  const appCode = search.app_code;
  const oauthError = search.error;
  const nextPath = getSafeNextPath(search.next);

  useEffect(() => {
    const complete = async () => {
      if (oauthError) {
        setError(t('remoteAuth.oauthError', { error: oauthError }));
        return;
      }

      if (!handoffId || !appCode) {
        return;
      }

      try {
        const verifier = retrieveVerifier();
        if (!verifier) {
          setError(t('remoteAuth.sessionLost'));
          return;
        }

        const { access_token, refresh_token } = await redeemOAuth(
          handoffId,
          appCode,
          verifier
        );

        await storeTokens(access_token, refresh_token);
        clearVerifier();

        window.location.replace(nextPath);
      } catch (e) {
        setError(e instanceof Error ? e.message : t('remoteAuth.loginFailed'));
        clearVerifier();
      }
    };

    void complete();
  }, [handoffId, appCode, oauthError, nextPath, t]);

  if (error) {
    return (
      <RemoteStatusCard title={t('remoteAuth.loginFailed')} variant="error">
        <p className="text-sm text-normal mt-base">{error}</p>
        <button
          type="button"
          className="agentos-button agentos-button--primary mt-double w-full"
          onClick={() =>
            navigate({
              to: '/account',
              search: nextPath !== '/' ? { next: nextPath } : undefined,
              replace: true,
            })
          }
        >
          {t('remoteAuth.tryAgain')}
        </button>
      </RemoteStatusCard>
    );
  }

  return (
    <RemoteStatusCard title={t('remoteAuth.completingLogin')}>
      <p className="text-sm text-low mt-base">
        {t('remoteAuth.processingOAuth')}
      </p>
    </RemoteStatusCard>
  );
}
