import { useState } from 'react';
import { ACTION_TYPES, type ActionType, type KeyAction } from '../types';
import { ActionTypeField } from './ActionTypeField';

// Nesting a multi-action inside itself would only add confusing recursion, not capability.
const ROW_ACTION_TYPES = ACTION_TYPES.filter((t) => t.value !== 'multi');

interface ActionEditorPopoverProps {
  index: number;
  action: KeyAction | undefined;
  disabled: boolean;
  onSubmit: (action: KeyAction) => void;
  onClose: () => void;
}

/** Floating side panel for editing a single row of a `MultiActionField`. */
export function ActionEditorPopover({
  index,
  action,
  disabled,
  onSubmit,
  onClose,
}: ActionEditorPopoverProps) {
  const [actionType, setActionType] = useState<ActionType>(
    action?.type ?? 'run_command',
  );
  // Bumped when the row's type changes, to remount the field and reset its
  // now-stale local input state (same trick as the top-level ActionSection).
  const [typeNonce, setTypeNonce] = useState(0);

  return (
    <div className="absolute left-full top-0 z-10 ml-3 flex w-[280px] flex-col gap-2 rounded-[10px] border border-border bg-bg p-3 shadow-modal">
      <div className="flex items-center justify-between">
        <span className="text-[13px] text-text-h">Action {index + 1}</span>
        <button
          type="button"
          className="cursor-pointer border-none bg-transparent text-base leading-none text-text-h"
          onClick={onClose}
          aria-label="Close"
        >
          ×
        </button>
      </div>

      <select
        className="rounded-sm border border-border bg-bg px-2 py-1 text-sm text-text-h"
        value={actionType}
        disabled={disabled}
        onChange={(e) => {
          setActionType(e.target.value as ActionType);
          setTypeNonce((n) => n + 1);
        }}
      >
        {ROW_ACTION_TYPES.map((t) => (
          <option key={t.value} value={t.value}>
            {t.label}
          </option>
        ))}
      </select>

      <ActionTypeField
        key={typeNonce}
        actionType={actionType}
        action={action}
        disabled={disabled}
        onSubmit={onSubmit}
      />
    </div>
  );
}
