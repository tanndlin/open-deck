import { useState } from 'react';
import type { KeyAction } from '../types';
import { ActionEditorPopover } from './ActionEditorPopover';
import { ActionRow } from './ActionRow';
import type { ActionFieldProps } from './types';

let nextRowId = 0;

interface Row {
  id: number;
  /** `undefined` for a freshly added row that hasn't been given an action yet. */
  action: KeyAction | undefined;
}

function seedRows(action: KeyAction | undefined): Row[] {
  const actions = action?.type === 'multi' ? action.actions : [];
  return actions.map((a) => ({ id: nextRowId++, action: a }));
}

export function MultiActionField({
  action,
  disabled,
  onSubmit,
}: ActionFieldProps) {
  // Owns its row list locally, like every other action field owns its input
  // state — only re-seeded from `action` on mount, not on every re-render.
  const [rows, setRows] = useState<Row[]>(() => seedRows(action));
  const [dragIndex, setDragIndex] = useState<number | null>(null);
  // Row currently open in the side popover, if any.
  const [selectedIndex, setSelectedIndex] = useState<number | null>(null);

  function commit(newRows: Row[]) {
    setRows(newRows);
    const actions = newRows
      .map((r) => r.action)
      .filter((a): a is KeyAction => a !== undefined);
    onSubmit({ type: 'multi', actions });
  }

  function handleAdd() {
    setRows((rs) => [...rs, { id: nextRowId++, action: undefined }]);
    setSelectedIndex(rows.length);
  }

  function handleRemove(index: number) {
    commit(rows.filter((_, i) => i !== index));
    setSelectedIndex((s) => {
      if (s === null) return s;
      if (s === index) return null;
      return s > index ? s - 1 : s;
    });
  }

  function handleDrop(targetIndex: number) {
    if (dragIndex === null || dragIndex === targetIndex) {
      setDragIndex(null);
      return;
    }
    const reordered = [...rows];
    const [moved] = reordered.splice(dragIndex, 1);
    reordered.splice(targetIndex, 0, moved);
    setDragIndex(null);
    setSelectedIndex((s) => (s === dragIndex ? targetIndex : s));
    commit(reordered);
  }

  const selectedRow = selectedIndex !== null ? rows[selectedIndex] : undefined;

  return (
    <div className="relative flex flex-col gap-2">
      {rows.length === 0 && (
        <p className="text-[13px] text-text opacity-70">No actions yet</p>
      )}

      <div className="flex flex-col gap-1">
        {rows.map((row, index) => (
          <ActionRow
            key={row.id}
            index={index}
            action={row.action}
            disabled={disabled}
            selected={selectedIndex === index}
            onSelect={() =>
              setSelectedIndex((s) => (s === index ? null : index))
            }
            onRemove={() => handleRemove(index)}
            onDragStart={() => setDragIndex(index)}
            onDragEnd={() => setDragIndex(null)}
            onDrop={() => handleDrop(index)}
          />
        ))}
      </div>

      <button
        type="button"
        className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
        disabled={disabled}
        onClick={handleAdd}
      >
        + Add action
      </button>

      {selectedRow && selectedIndex !== null && (
        <ActionEditorPopover
          index={selectedIndex}
          action={selectedRow.action}
          disabled={disabled}
          onClose={() => setSelectedIndex(null)}
          onSubmit={(sub) =>
            commit(
              rows.map((r, i) =>
                i === selectedIndex ? { ...r, action: sub } : r,
              ),
            )
          }
        />
      )}
    </div>
  );
}
