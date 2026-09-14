import type { StatusItem } from '@/shared/types/selectionItems';
import i18n from '@/i18n';
import type { SelectionPage } from '../SelectionDialog';

export interface StatusSelectionResult {
  statusId: string;
}

export function buildStatusSelectionPages(
  statuses: StatusItem[]
): Record<string, SelectionPage<StatusSelectionResult>> {
  return {
    selectStatus: {
      id: 'selectStatus',
      title: i18n.t('commandBar.selectionTitles.status'),
      buildGroups: () => [
        {
          label: i18n.t('commandBar.selectionGroups.statuses'),
          items: statuses.map((s) => ({ type: 'status' as const, status: s })),
        },
      ],
      onSelect: (item) => {
        if (item.type === 'status') {
          return {
            type: 'complete',
            data: { statusId: item.status.id },
          };
        }
        return { type: 'complete', data: undefined as never };
      },
    },
  };
}
