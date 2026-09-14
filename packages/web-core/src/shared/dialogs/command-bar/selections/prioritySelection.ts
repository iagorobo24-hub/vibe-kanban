import type { IssuePriority } from 'shared/remote-types';
import i18n from '@/i18n';
import type { PriorityItem } from '@/shared/types/selectionItems';
import type { SelectionPage } from '../SelectionDialog';

export interface PrioritySelectionResult {
  priority: IssuePriority | null;
}

function getPriorityItems(): PriorityItem[] {
  return [
    { id: null, name: i18n.t('kanban.noPriority') },
    { id: 'urgent', name: i18n.t('kanban.priorityLevels.urgent') },
    { id: 'high', name: i18n.t('kanban.priorityLevels.high') },
    { id: 'medium', name: i18n.t('kanban.priorityLevels.medium') },
    { id: 'low', name: i18n.t('kanban.priorityLevels.low') },
  ];
}

export function buildPrioritySelectionPages(): Record<
  string,
  SelectionPage<PrioritySelectionResult>
> {
  return {
    selectPriority: {
      id: 'selectPriority',
      title: i18n.t('commandBar.selectionTitles.priority'),
      buildGroups: () => [
        {
          label: i18n.t('commandBar.selectionGroups.priority'),
          items: getPriorityItems().map((p) => ({
            type: 'priority' as const,
            priority: p,
          })),
        },
      ],
      onSelect: (item) => {
        if (item.type === 'priority') {
          return {
            type: 'complete',
            data: { priority: item.priority.id },
          };
        }
        return { type: 'complete', data: undefined as never };
      },
    },
  };
}
