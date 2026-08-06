import type { DragEvent } from 'react';
import { ACTION_TYPES, actionSummary, type KeyAction } from '../types';

interface ActionRowProps {
  index: number;
  action: KeyAction | undefined;
  disabled: boolean;
  selected: boolean;
  onSelect: () => void;
  onRemove: () => void;
  onDragStart: () => void;
  onDragEnd: () => void;
  onDrop: () => void;
}

/** One compact row of a `MultiActionField`'s list. Click it to edit in the side popover. */
export function ActionRow({
  index,
  action,
  disabled,
  selected,
  onSelect,
  onRemove,
  onDragStart,
  onDragEnd,
  onDrop,
}: ActionRowProps) {
  const typeLabel =
    ACTION_TYPES.find((t) => t.value === action?.type)?.label ?? 'Unset';

  return (
    <div
      className={`flex cursor-pointer items-center gap-2 rounded-sm border px-2 py-1.5 text-left ${
        selected ? 'border-accent bg-accent-bg' : 'border-border'
      }`}
      // Whole row (not just the handle) is the drag source, so the native drag image ghosts the row like a key tile.
      draggable
      onDragStart={(e: DragEvent) => {
        e.dataTransfer.effectAllowed = 'move';
        onDragStart();
      }}
      onDragEnd={onDragEnd}
      onDragOver={(e: DragEvent) => e.preventDefault()}
      onDrop={(e: DragEvent) => {
        e.preventDefault();
        onDrop();
      }}
      onClick={onSelect}
    >
      <span
        className="cursor-grab select-none text-text opacity-60"
        aria-label="Drag to reorder"
      >
        ⠿
      </span>
      <span className="font-mono text-xs text-text-h opacity-60">
        {index + 1}.
      </span>
      <span className="shrink-0 text-[13px] text-text-h">{typeLabel}</span>
      <span className="flex-1 truncate font-mono text-xs text-text opacity-70">
        {actionSummary(action)}
      </span>
      <button
        type="button"
        className="cursor-pointer border-none bg-transparent text-base leading-none text-text-h disabled:cursor-not-allowed disabled:opacity-50"
        disabled={disabled}
        onClick={(e) => {
          e.stopPropagation();
          onRemove();
        }}
        aria-label="Remove action"
      >
        ×
      </button>
    </div>
  );
}
