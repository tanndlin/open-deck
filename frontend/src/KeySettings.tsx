import { useEffect, useState } from 'react';
import { keyImageUrl, type KeyAction, type KeyConfig } from './api';

interface KeySettingsProps {
  id: number;
  config: KeyConfig;
  version: number;
  onClose: () => void;
  onSetIcon: (id: number, path: string) => Promise<void>;
  onClearIcon: (id: number) => Promise<void>;
  onSetAction: (id: number, action: KeyAction) => Promise<void>;
  onClearAction: (id: number) => Promise<void>;
}

// Action kinds the UI knows how to edit. Add a case here (and to the switch
// in the action editor below) for each new `KeyAction` variant.
const ACTION_TYPES = [{ value: 'run_command', label: 'Run command' }] as const;

export function KeySettings({
  id,
  config,
  version,
  onClose,
  onSetIcon,
  onClearIcon,
  onSetAction,
  onClearAction,
}: KeySettingsProps) {
  const { icon: path, action } = config;
  const [draft, setDraft] = useState(path ?? '');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [previewBroken, setPreviewBroken] = useState(false);

  const [actionType, setActionType] = useState<KeyAction['type']>(
    action?.type ?? 'run_command',
  );
  const [command, setCommand] = useState(
    action?.type === 'run_command' ? action.command : '',
  );

  useEffect(() => setPreviewBroken(false), [path, version]);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose();
    }
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [onClose]);

  async function run(action: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  function buildAction(): KeyAction {
    switch (actionType) {
      case 'run_command':
        return { type: 'run_command', command: command.trim() };
    }
  }

  const actionValid =
    actionType === 'run_command' ? command.trim() !== '' : false;

  return (
    <div
      className="fixed inset-0 z-10 flex items-center justify-center bg-black/50"
      onClick={onClose}
    >
      <div
        className="flex w-[min(360px,calc(100vw-32px))] flex-col gap-3.5 rounded-[10px] border border-border bg-bg p-5 shadow-modal"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="flex items-center justify-between">
          <h2 className="m-0">Key {id}</h2>
          <button
            type="button"
            className="cursor-pointer border-none bg-transparent p-1 text-xl leading-none text-text-h"
            onClick={onClose}
            aria-label="Close"
          >
            ×
          </button>
        </div>

        <div className="mx-auto flex aspect-square w-35 items-center justify-center overflow-hidden rounded-[10px] border border-border bg-code-bg">
          {path !== undefined && !previewBroken ? (
            <img
              className="h-full w-full object-cover"
              src={keyImageUrl(id, version)}
              alt={`Key ${id}`}
              onError={() => setPreviewBroken(true)}
            />
          ) : (
            <span className="px-3 text-center text-[13px] text-text">
              No icon set
            </span>
          )}
        </div>

        <label className="flex flex-col gap-1.5 text-[13px] text-text">
          Icon path
          <input
            type="text"
            className="rounded-sm border border-border bg-bg px-2 py-1.5 font-mono text-sm text-text-h"
            value={draft}
            placeholder="icons/example.png"
            disabled={busy}
            autoFocus
            onChange={(e) => setDraft(e.target.value)}
          />
        </label>

        <div className="flex gap-2">
          <button
            type="button"
            className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
            disabled={busy || draft.trim() === ''}
            onClick={() => run(() => onSetIcon(id, draft.trim()))}
          >
            Set icon
          </button>
          <button
            type="button"
            className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
            disabled={busy || path === undefined}
            onClick={() => run(() => onClearIcon(id).then(() => setDraft('')))}
          >
            Clear icon
          </button>
        </div>

        <hr className="w-full border-border" />

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

        {actionType === 'run_command' && (
          <label className="flex flex-col gap-1.5 text-[13px] text-text">
            Command
            <input
              type="text"
              className="rounded-sm border border-border bg-bg px-2 py-1.5 font-mono text-sm text-text-h"
              value={command}
              placeholder="python script.py"
              disabled={busy}
              onChange={(e) => setCommand(e.target.value)}
            />
          </label>
        )}

        {error && <div className="text-[13px] text-[#e5484d]">{error}</div>}

        <div className="flex gap-2">
          <button
            type="button"
            className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
            disabled={busy || !actionValid}
            onClick={() => run(() => onSetAction(id, buildAction()))}
          >
            Set action
          </button>
          <button
            type="button"
            className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
            disabled={busy || action === undefined}
            onClick={() =>
              run(() => onClearAction(id).then(() => setCommand('')))
            }
          >
            Clear action
          </button>
          <button
            type="button"
            className="ml-auto cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
            onClick={onClose}
            disabled={busy}
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
