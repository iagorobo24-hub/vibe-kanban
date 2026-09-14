import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import {
  StackIcon,
  SlidersIcon,
  SquaresFourIcon,
  GitBranchIcon,
  KanbanIcon,
} from '@phosphor-icons/react';
import type { Workspace } from 'shared/types';
import { Pages } from '@/shared/command-bar/actions/pages';
import type {
  PageId,
  StaticPageId,
  CommandBarGroupItem,
  ResolvedGroup,
  ResolvedGroupItem,
} from '@/shared/types/commandBar';
import {
  isActionVisible,
  type ActionDefinition,
  type ActionVisibilityContext,
} from '@/shared/types/actions';
import { isPageVisible } from '@/shared/command-bar/actions/useActionVisibility';
import { injectSearchMatches } from './injectSearchMatches';

export interface ResolvedCommandBarPage {
  id: string;
  title?: string;
  groups: ResolvedGroup[];
}

const PAGE_ICONS = {
  root: SquaresFourIcon,
  workspaceActions: StackIcon,
  diffOptions: SlidersIcon,
  viewOptions: SquaresFourIcon,
  repoActions: GitBranchIcon,
  issueActions: KanbanIcon,
} as const satisfies Record<StaticPageId, typeof StackIcon>;

const PAGE_TITLE_KEYS: Partial<Record<StaticPageId, string>> = {
  workspaceActions: 'commandBar.pages.workspaceActions',
  diffOptions: 'commandBar.pages.diffOptions',
  viewOptions: 'commandBar.pages.viewOptions',
  repoActions: 'commandBar.pages.repoActions',
  issueActions: 'commandBar.pages.issueActions',
};

const GROUP_LABEL_KEYS: Record<string, string> = {
  Actions: 'commandBar.groups.actions',
  View: 'commandBar.groups.view',
  General: 'commandBar.groups.general',
  Workspace: 'commandBar.groups.workspace',
  Scripts: 'commandBar.groups.scripts',
  Display: 'commandBar.groups.display',
  Panels: 'commandBar.groups.panels',
};

function translateCommandBarText(
  value: string,
  t: (key: string, options?: { defaultValue?: string }) => string
) {
  const key = GROUP_LABEL_KEYS[value];
  return key ? t(key, { defaultValue: value }) : value;
}

function expandGroupItems(
  items: CommandBarGroupItem[],
  ctx: ActionVisibilityContext,
  t: (key: string, options?: { defaultValue?: string }) => string
): ResolvedGroupItem[] {
  return items.flatMap((item) => {
    if (item.type === 'childPages') {
      const page = Pages[item.id as StaticPageId];
      if (!isPageVisible(page, ctx)) return [];
      return [
        {
          type: 'page' as const,
          pageId: item.id,
          label: page.title
            ? t(PAGE_TITLE_KEYS[item.id] ?? '', {
                defaultValue: page.title,
              })
            : item.id,
          icon: PAGE_ICONS[item.id as StaticPageId],
        },
      ];
    }
    if (item.type === 'action') {
      if (!isActionVisible(item.action, ctx)) return [];
    }
    return [item];
  });
}

function buildPageGroups(
  pageId: StaticPageId,
  ctx: ActionVisibilityContext,
  t: (key: string, options?: { defaultValue?: string }) => string
): ResolvedGroup[] {
  return Pages[pageId].items
    .map((group) => {
      const items = expandGroupItems(group.items, ctx, t);
      return items.length
        ? { label: translateCommandBarText(group.label, t), items }
        : null;
    })
    .filter((g): g is ResolvedGroup => g !== null);
}

export function useResolvedPage(
  pageId: PageId,
  search: string,
  ctx: ActionVisibilityContext,
  workspace: Workspace | undefined,
  getLabel: (
    action: ActionDefinition,
    workspace?: Workspace,
    ctx?: ActionVisibilityContext
  ) => string
): ResolvedCommandBarPage {
  const { t } = useTranslation('common');

  return useMemo(() => {
    const groups = buildPageGroups(pageId, ctx, t);
    if (pageId === 'root' && search.trim()) {
      groups.push(...injectSearchMatches(search, ctx, workspace, getLabel));
    }

    return {
      id: Pages[pageId].id,
      title: Pages[pageId].title
        ? t(PAGE_TITLE_KEYS[pageId] ?? '', {
            defaultValue: Pages[pageId].title,
          })
        : undefined,
      groups,
    };
  }, [pageId, search, ctx, workspace, getLabel, t]);
}
