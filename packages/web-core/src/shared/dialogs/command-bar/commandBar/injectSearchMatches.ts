import type { Workspace } from 'shared/types';
import { Pages, getPageActions } from '@/shared/command-bar/actions/pages';
import type { StaticPageId, ResolvedGroup } from '@/shared/types/commandBar';
import {
  isActionVisible,
  type ActionDefinition,
  type ActionVisibilityContext,
} from '@/shared/types/actions';

type ActionLabelResolver = (
  action: ActionDefinition,
  workspace?: Workspace,
  ctx?: ActionVisibilityContext
) => string;

// Derive injectable pages from Pages - all child pages of root
const INJECTABLE_PAGE_IDS = (Object.keys(Pages) as StaticPageId[]).filter(
  (id) => id !== 'root' && Pages[id].parent === 'root'
);

export function injectSearchMatches(
  searchQuery: string,
  ctx: ActionVisibilityContext,
  workspace: Workspace | undefined,
  getLabel: ActionLabelResolver
): ResolvedGroup[] {
  const searchLower = searchQuery.toLowerCase();

  return INJECTABLE_PAGE_IDS.reduce<ResolvedGroup[]>((groups, id) => {
    const page = Pages[id];

    // Check page visibility condition
    if (page.isVisible && !page.isVisible(ctx)) return groups;

    const items = getPageActions(id)
      .filter((a) => isActionVisible(a, ctx))
      .filter((a) => {
        const label = getLabel(a, workspace, ctx);
        return (
          label.toLowerCase().includes(searchLower) ||
          a.id.toLowerCase().includes(searchLower) ||
          (a.keywords?.some((kw) => kw.toLowerCase().includes(searchLower)) ??
            false)
        );
      })
      .map((action) => ({ type: 'action' as const, action }));

    if (items.length) groups.push({ label: page.title || id, items });
    return groups;
  }, []);
}
