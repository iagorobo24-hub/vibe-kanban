import { useCallback } from 'react';
import { useRouter } from '@tanstack/react-router';
import { BellIcon, CheckIcon, ChecksIcon } from '@phosphor-icons/react';
import { UserAvatar } from '@vibe/ui/components/UserAvatar';
import { useNotifications } from '@/shared/hooks/useNotifications';
import { useNotificationMembers } from '@/shared/hooks/useNotificationMembers';
import type { GroupedNotification } from '@/shared/lib/notifications';
import {
  getGroupedNotificationSegments,
  type MessageSegment,
} from '@/shared/lib/notificationMessage';
import { formatRelativeTime } from '@/shared/lib/date';
import { cn } from '@/shared/lib/utils';

function NotificationMessage({
  segments,
  membersByUserId,
}: {
  segments: MessageSegment[];
  membersByUserId: ReturnType<typeof useNotificationMembers>['membersByUserId'];
}) {
  return (
    <>
      {segments.map((seg, i) => {
        if (seg.type === 'text') return <span key={i}>{seg.value}</span>;
        if (seg.type === 'emphasis') {
          return (
            <span key={i} className="font-medium text-high">
              {seg.value}
            </span>
          );
        }
        if (seg.type === 'issue') {
          return (
            <span
              key={i}
              className="font-ibm-plex-mono text-high text-[0.95em]"
            >
              {seg.value}
            </span>
          );
        }
        const member = membersByUserId.get(seg.userId);
        if (member) {
          return (
            <UserAvatar
              key={i}
              user={member}
              className="inline-flex h-5 w-5 align-text-bottom text-[10px]"
            />
          );
        }
        return <span key={i}>Someone</span>;
      })}
    </>
  );
}

export function NotificationsPage() {
  const router = useRouter();
  const { data, updateMany, enabled, unseenCount, groupedNotifications } =
    useNotifications();
  const { membersByUserId } = useNotificationMembers(data);

  const markGroupSeen = useCallback(
    (group: GroupedNotification) => {
      if (group.unseenNotificationIds.length === 0) {
        return;
      }

      updateMany(
        group.unseenNotificationIds.map((notificationId) => ({
          id: notificationId,
          changes: { seen: true },
        }))
      );
    },
    [updateMany]
  );

  const handleClick = useCallback(
    (group: GroupedNotification) => {
      markGroupSeen(group);
      const path = group.deeplinkPath;
      if (path) {
        router.navigate({ to: path as '/' });
      }
    },
    [markGroupSeen, router]
  );

  const handleMarkAllSeen = useCallback(() => {
    const unseen = data.filter((n) => !n.seen);
    if (unseen.length === 0) return;
    updateMany(unseen.map((n) => ({ id: n.id, changes: { seen: true } })));
  }, [data, updateMany]);

  if (!enabled) {
    return (
      <div
        className="agentos-theme agentos-page-shell agentos-empty-state h-full"
        role="status"
        aria-labelledby="agentos-notifications-auth-title"
      >
        <BellIcon size={32} weight="light" aria-hidden="true" />
        <h1
          id="agentos-notifications-auth-title"
          className="agentos-empty-state__title"
        >
          Notifications
        </h1>
        <p className="agentos-empty-state__description">
          Sign in to view notifications from your agents and workspaces.
        </p>
      </div>
    );
  }

  return (
    <div className="agentos-notifications-page flex flex-col h-full overflow-hidden">
      <div className="agentos-notifications-page__header flex items-center justify-between px-double py-base border-b border-border">
        <h1 className="text-xl font-medium text-high">Notifications</h1>
        {unseenCount > 0 && (
          <button
            type="button"
            onClick={handleMarkAllSeen}
            className="agentos-notifications-page__mark-all flex items-center gap-1 px-base py-half text-sm text-low hover:text-normal transition-colors cursor-pointer"
          >
            <ChecksIcon size={16} />
            Mark all as read
          </button>
        )}
      </div>

      <div className="agentos-notifications-page__list flex-1 overflow-y-auto">
        {groupedNotifications.length === 0 ? (
          <div className="agentos-notifications-page__empty agentos-empty-state h-full">
            <BellIcon size={32} weight="light" aria-hidden="true" />
            <p className="agentos-empty-state__title">No notifications yet</p>
            <p className="agentos-empty-state__description">
              Updates that need your attention will appear here.
            </p>
          </div>
        ) : (
          <div className="divide-y divide-border">
            {groupedNotifications.map((group) => (
              <article
                key={group.id}
                className={cn(
                  'agentos-notifications-page__row w-full flex items-center gap-base px-double py-base transition-colors',
                  'hover:bg-secondary',
                  !group.seen && 'bg-brand/5'
                )}
              >
                <button
                  type="button"
                  onClick={() => handleClick(group)}
                  className="agentos-notifications-page__open flex min-w-0 flex-1 items-center gap-base text-left focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-inset focus-visible:ring-brand"
                >
                  <span
                    className={cn(
                      'agentos-notifications-page__unread-dot shrink-0 w-2 h-2 rounded-full',
                      !group.seen && 'bg-brand'
                    )}
                    aria-hidden="true"
                  />
                  <span className="flex min-w-0 flex-1 flex-col">
                    <span
                      className={cn(
                        'agentos-notifications-page__message text-base truncate',
                        group.seen ? 'text-normal' : 'text-high'
                      )}
                    >
                      <NotificationMessage
                        segments={getGroupedNotificationSegments(group)}
                        membersByUserId={membersByUserId}
                      />
                    </span>
                    <span className="agentos-notifications-page__time text-sm text-low mt-0.5">
                      {formatRelativeTime(group.latest.created_at)}
                    </span>
                  </span>
                </button>
                {!group.seen && (
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      markGroupSeen(group);
                    }}
                    onKeyDown={(e) => e.stopPropagation()}
                    className={cn(
                      'agentos-notifications-page__mark shrink-0 inline-flex items-center gap-half rounded-sm px-half py-half text-sm text-low transition-colors cursor-pointer',
                      'hover:bg-secondary hover:text-normal',
                      'focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand'
                    )}
                    aria-label="Mark notification as read"
                    title="Mark as read"
                  >
                    <CheckIcon size={14} weight="bold" />
                    <span className="hidden sm:inline">Mark as read</span>
                  </button>
                )}
              </article>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
