'use client';

import { Card } from './Card';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from './RadixTooltip';
import { cn } from '../lib/cn';
import {
  DragDropContext,
  Droppable,
  Draggable,
  type DropResult,
  type DraggableProvided,
  type DraggableStateSnapshot,
  type DroppableProvided,
} from '@hello-pangea/dnd';
import {
  type KeyboardEvent,
  type MouseEvent,
  type MutableRefObject,
  type ReactNode,
  type Ref,
} from 'react';
import { useTranslation } from 'react-i18next';
import { DotsSixVerticalIcon, PlusIcon } from '@phosphor-icons/react';
import { Button } from './Button';

export type { DropResult } from '@hello-pangea/dnd';

export type Status = {
  id: string;
  name: string;
  color: string;
};

export type Feature = {
  id: string;
  name: string;
  startAt: Date;
  endAt: Date;
  status: Status;
};

// =============================================================================
// Kanban Board (Droppable Column)
// =============================================================================

export type KanbanBoardProps = {
  children: ReactNode;
  className?: string;
};

export const KanbanBoard = ({ children, className }: KanbanBoardProps) => {
  return (
    <div
      className={cn('agentos-kanban-column flex flex-col min-h-40', className)}
    >
      {children}
    </div>
  );
};

// =============================================================================
// Kanban Card (Draggable)
// =============================================================================

export type KanbanCardProps = Pick<Feature, 'id' | 'name'> & {
  index: number;
  children?: ReactNode;
  className?: string;
  onClick?: (e: MouseEvent<HTMLDivElement>) => void;
  onActivate?: () => void;
  tabIndex?: number;
  forwardedRef?: Ref<HTMLDivElement>;
  onKeyDown?: (e: KeyboardEvent) => void;
  isOpen?: boolean;
  isSelected?: boolean;
  dragDisabled?: boolean;
  isMobile?: boolean;
};

export const KanbanCard = ({
  id,
  name,
  index,
  children,
  className,
  onClick,
  onActivate,
  tabIndex,
  forwardedRef,
  onKeyDown,
  isOpen,
  isSelected,
  dragDisabled = false,
  isMobile,
}: KanbanCardProps) => {
  const { t } = useTranslation('common');

  return (
    <Draggable draggableId={id} index={index} isDragDisabled={dragDisabled}>
      {(provided: DraggableProvided, snapshot: DraggableStateSnapshot) => {
        // Combine DnD ref and forwarded ref
        const setRefs = (node: HTMLDivElement | null) => {
          provided.innerRef(node);
          if (typeof forwardedRef === 'function') {
            forwardedRef(node);
          } else if (forwardedRef && typeof forwardedRef === 'object') {
            (forwardedRef as MutableRefObject<HTMLDivElement | null>).current =
              node;
          }
        };

        const handleCardKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
          onKeyDown?.(event);

          if (
            event.defaultPrevented ||
            isMobile ||
            !onActivate ||
            event.key !== 'Enter'
          ) {
            return;
          }

          event.preventDefault();
          onActivate();
        };

        return (
          <Card
            className={cn(
              'agentos-kanban-card p-base flex-col border -mt-[1px] -mx-[1px] bg-primary focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-brand focus-visible:ring-inset',
              snapshot.isDragging && 'cursor-grabbing shadow-lg',
              isSelected
                ? 'ring-2 ring-accent ring-inset bg-accent/5'
                : isOpen && 'ring-2 ring-secondary-foreground ring-inset',
              className
            )}
            ref={setRefs}
            {...provided.draggableProps}
            {...(isMobile ? {} : provided.dragHandleProps)}
            {...(tabIndex !== undefined ? { tabIndex } : {})}
            onMouseUp={
              !isMobile
                ? (e) => {
                    if (e.button === 0 && !snapshot.isDragging) {
                      onClick?.(e);
                    }
                  }
                : undefined
            }
            onKeyDown={isMobile ? undefined : handleCardKeyDown}
          >
            {isMobile ? (
              <div className="flex gap-half">
                <div
                  {...provided.dragHandleProps}
                  className="flex items-start pt-half cursor-grab shrink-0 rounded-sm focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-brand"
                  onClick={(e) => e.stopPropagation()}
                  aria-label={t('accessibility.reorderColumn', { name })}
                >
                  <DotsSixVerticalIcon
                    className="size-icon-xs text-low"
                    weight="bold"
                    aria-hidden="true"
                  />
                </div>
                <div className="flex-1 min-w-0">
                  {children ?? (
                    <p className="m-0 font-medium text-sm">{name}</p>
                  )}
                </div>
              </div>
            ) : (
              (children ?? <p className="m-0 font-medium text-sm">{name}</p>)
            )}
          </Card>
        );
      }}
    </Draggable>
  );
};

// =============================================================================
// Kanban Cards Container
// =============================================================================

export type KanbanCardsProps = {
  id: string;
  children: ReactNode;
  className?: string;
};

export const KanbanCards = ({ id, children, className }: KanbanCardsProps) => (
  <Droppable droppableId={id}>
    {(provided: DroppableProvided) => (
      <div
        className={cn('flex flex-1 flex-col', className)}
        ref={provided.innerRef}
        {...provided.droppableProps}
      >
        {children}
        {provided.placeholder}
      </div>
    )}
  </Droppable>
);

// =============================================================================
// Kanban Header
// =============================================================================

export type KanbanHeaderProps =
  | {
      children: ReactNode;
    }
  | {
      name: Status['name'];
      color: Status['color'];
      className?: string;
      onAddTask?: () => void;
    };

export const KanbanHeader = (props: KanbanHeaderProps) => {
  const { t } = useTranslation('tasks');

  if ('children' in props) {
    return props.children;
  }

  return (
    <Card
      className={cn(
        'sticky top-0 z-20 flex shrink-0 items-center gap-base p-base flex gap-base',
        'bg-background',
        props.className
      )}
      style={{
        backgroundImage: `linear-gradient(hsl(var(${props.color}) / 0.03), hsl(var(${props.color}) / 0.03))`,
      }}
    >
      <span className="flex-1 flex items-center gap-base">
        <div
          className="h-2 w-2 rounded-full"
          style={{ backgroundColor: `hsl(var(${props.color}))` }}
        />

        <p className="m-0 text-sm">{props.name}</p>
      </span>
      <TooltipProvider>
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon"
              className="agentos-kanban-column__add m-0 text-foreground/50 hover:text-foreground"
              onClick={props.onAddTask}
              aria-label={t('actions.addTask')}
            >
              <PlusIcon className="h-4 w-4" aria-hidden="true" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="top">{t('actions.addTask')}</TooltipContent>
        </Tooltip>
      </TooltipProvider>
    </Card>
  );
};

// =============================================================================
// Kanban Provider (DragDropContext)
// =============================================================================

export type KanbanProviderProps = {
  children: ReactNode;
  onDragEnd: (result: DropResult) => void;
  className?: string;
};

export const KanbanProvider = ({
  children,
  onDragEnd,
  className,
}: KanbanProviderProps) => {
  return (
    <DragDropContext onDragEnd={onDragEnd}>
      <div
        className={cn(
          'agentos-kanban-provider inline-grid grid-flow-col auto-cols-[minmax(200px,400px)] divide-x border-x items-stretch min-h-full',
          className
        )}
      >
        {children}
      </div>
    </DragDropContext>
  );
};
