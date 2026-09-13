import { useUserSystem } from '@/shared/hooks/useUserSystem';

/**
 * Remote authentication is an optional capability in the local-first app.
 * Keep the degraded reason available to callers so they can render an
 * actionable state instead of treating an unavailable remote plane as an
 * empty or indefinitely loading result.
 */
export function useRemoteAuthAvailability() {
  const { loading, remoteAuthDegraded } = useUserSystem();

  return {
    remoteAuthDegraded,
    // Do not let remote subscriptions race the initial /api/info load. The
    // local plane must finish establishing its capabilities before callers
    // can decide whether the optional remote plane is usable.
    isRemoteAuthAvailable: !loading && !remoteAuthDegraded,
  };
}
