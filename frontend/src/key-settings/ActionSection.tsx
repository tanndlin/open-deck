import { useState } from 'react';
import { ActionTypeField } from '../action-fields/ActionTypeField';
import { ACTION_TYPES, type ActionType, type KeyAction } from '../types';

interface ActionSectionProps {
  id: number;
  action: KeyAction | undefined;
  busy: boolean;
  run: (fn: () => Promise<void>) => void;
  onSetAction: (id: number, action: KeyAction) => Promise<void>;
  onClearAction: (id: number) => Promise<void>;
  onMakeFolder: (id: number) => Promise<void>;
}

export function ActionSection({
  id,
  action,
  busy,
  run,
  onSetAction,
  onClearAction,
  onMakeFolder,
}: ActionSectionProps) {
  const [actionType, setActionType] = useState<ActionType>(
    action?.type ?? 'run_command',
  );
  // Bumped after clearing the action to force the currently selected
  // ActionTypeField to remount and reset its own (now-owned) input state.
  const [clearNonce, setClearNonce] = useState(0);

  return (
    <>
      <label className="flex flex-col gap-1.5 text-[13px] text-text">
        Action on press
        <select
          className="rounded-sm border border-border bg-bg px-2 py-1.5 text-sm text-text-h"
          value={actionType}
          disabled={busy}
          onChange={(e) => setActionType(e.target.value as KeyAction['type'])}
        >
          {ACTION_TYPES.map((t) => (
            <option key={t.value} value={t.value}>
              {t.label}
            </option>
          ))}
        </select>
      </label>

      <ActionTypeField
        key={clearNonce}
        actionType={actionType}
        action={action}
        disabled={busy}
        onSubmit={(builtAction) => run(() => onSetAction(id, builtAction))}
      />

      <button
        type="button"
        className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
        disabled={busy || action === undefined}
        onClick={() =>
          run(() => onClearAction(id).then(() => setClearNonce((n) => n + 1)))
        }
      >
        Clear action
      </button>

      <button
        type="button"
        className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
        disabled={busy}
        onClick={() => run(() => onMakeFolder(id))}
      >
        Make this a folder
      </button>
    </>
  );
}
