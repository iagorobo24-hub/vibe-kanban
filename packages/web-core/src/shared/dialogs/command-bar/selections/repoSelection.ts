import type { RepoItem } from '@/shared/types/selectionItems';
import i18n from '@/i18n';
import type { SelectionPage } from '../SelectionDialog';

export interface RepoSelectionResult {
  repoId: string;
}

export function buildRepoSelectionPages(
  repos: RepoItem[]
): Record<string, SelectionPage<RepoSelectionResult>> {
  return {
    selectRepo: {
      id: 'selectRepo',
      title: i18n.t('commandBar.selectionTitles.repository'),
      buildGroups: () => [
        {
          label: i18n.t('commandBar.selectionGroups.repositories'),
          items: repos.map((r) => ({ type: 'repo' as const, repo: r })),
        },
      ],
      onSelect: (item) => {
        if (item.type === 'repo') {
          return { type: 'complete', data: { repoId: item.repo.id } };
        }
        return { type: 'complete', data: undefined as never };
      },
    },
  };
}
