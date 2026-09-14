import { useEffect, useState } from 'react';
import { useTranslation } from '@/i18n/useTranslation';
import { useParams, useSearch } from '@tanstack/react-router';
import { acceptInvitation, redeemOAuth } from '@remote/shared/lib/api';
import { storeTokens } from '@remote/shared/lib/auth';
import {
  clearInvitationToken,
  clearVerifier,
  retrieveInvitationToken,
  retrieveVerifier,
} from '@remote/shared/lib/pkce';
import { RemoteStatusCard } from '@remote/shared/components/RemotePagePrimitives';

export default function InvitationCompletePage() {
  const { t } = useTranslation('common');
  const { token: urlToken } = useParams({
    from: '/invitations/$token/complete',
  });
  const search = useSearch({ from: '/invitations/$token/complete' });
  const [error, setError] = useState<string | null>(null);
  const [isAccepted, setIsAccepted] = useState(false);

  const handoffId = search.handoff_id;
  const appCode = search.app_code;
  const oauthError = search.error;

  useEffect(() => {
    const completeInvitation = async () => {
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

        const token = retrieveInvitationToken() || urlToken;
        if (!token) {
          setError(t('remoteAuth.invitationTokenLost'));
          return;
        }

        const { access_token, refresh_token } = await redeemOAuth(
          handoffId,
          appCode,
          verifier
        );

        await storeTokens(access_token, refresh_token);
        await acceptInvitation(token, access_token);

        clearVerifier();
        clearInvitationToken();

        setIsAccepted(true);
      } catch (e) {
        setError(
          e instanceof Error ? e.message : t('remoteAuth.invitationFailed')
        );
        clearVerifier();
        clearInvitationToken();
      }
    };

    void completeInvitation();
  }, [handoffId, appCode, oauthError, urlToken, t]);

  if (error) {
    const retryPath = urlToken ? `/invitations/${urlToken}/accept` : '/account';

    return (
      <RemoteStatusCard
        title={t('remoteAuth.couldNotAcceptInvitation')}
        variant="error"
      >
        <p className="mt-base text-sm text-normal">{error}</p>
        <button
          type="button"
          className="agentos-button agentos-button--primary mt-double w-full"
          onClick={() => {
            window.location.assign(retryPath);
          }}
        >
          {t('remoteAuth.tryAgain')}
        </button>
      </RemoteStatusCard>
    );
  }

  if (isAccepted) {
    return (
      <RemoteStatusCard title={t('remoteAuth.invitationAccepted')}>
        <p className="mt-base text-sm text-normal">
          {t('remoteAuth.invitationConfirmed')}
        </p>
        <a
          href="https://www.vibekanban.com/docs/getting-started"
          target="_blank"
          rel="noopener noreferrer"
          className="agentos-button agentos-button--primary mt-double w-full"
        >
          {t('remoteAuth.getStarted')}
        </a>
      </RemoteStatusCard>
    );
  }

  return (
    <RemoteStatusCard title={t('remoteAuth.completingInvitation')}>
      <p className="mt-base text-sm text-low">
        {t('remoteAuth.processingOAuth')}
      </p>
    </RemoteStatusCard>
  );
}
