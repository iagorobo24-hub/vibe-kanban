import { useUserSystem } from '@/shared/hooks/useUserSystem';

/**
 * Remote authentication is an optional capability in the local-first app.
 * Keep the degraded reason available to callers so they can render an
 * actionable state instead of treating an unavailable remote plane as an
 * empty or indefinitely loading result.
 */
export function useRemoteAuthAvailability() {
  const { remoteAuthDegraded } = useUserSystem();

  return {
    remoteAuthDegraded,
    isRemoteAuthAvailable: !remoteAuthDegraded,
  };
}
