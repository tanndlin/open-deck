import { useEffect, useState } from 'react';
import {
  keyImageUrl,
  type KeyAction,
  type KeyConfig,
  type PagePath,
} from './api';

interface KeySettingsProps {
  id: number;
  path: PagePath;
  config: KeyConfig;
  version: number;
  onClose: () => void;
  onSetIcon: (id: number, path: string) => Promise<void>;
  onClearIcon: (id: number) => Promise<void>;
  onSetAction: (id: number, action: KeyAction) => Promise<void>;
  onClearAction: (id: number) => Promise<void>;
  onMakeFolder: (id: number) => Promise<void>;
  onRemoveFolder: (id: number) => Promise<void>;
  onOpenFolder: (id: number) => void;
}

// Action kinds the UI knows how to edit. Add a case here (and to the switch
// in the action editor below) for each new `KeyAction` variant.
const ACTION_TYPES = [
  { value: 'run_command', label: 'Run command' },
  { value: 'open_url', label: 'Open webpage' },
] as const;

export function KeySettings({
  id,
  path,
  config,
  version,
  onClose,
  onSetIcon,
  onClearIcon,
  onSetAction,
  onClearAction,
  onMakeFolder,
  onRemoveFolder,
  onOpenFolder,
}: KeySettingsProps) {
  const { icon, action, is_folder: isFolder } = config;
  const [draft, setDraft] = useState(icon ?? '');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [previewBroken, setPreviewBroken] = useState(false);

  const [actionType, setActionType] = useState<KeyAction['type']>(
    action?.type ?? 'run_command',
  );
  const [command, setCommand] = useState(
    action?.type === 'run_command' ? action.command : '',
  );
  const [url, setUrl] = useState(action?.type === 'open_url' ? action.url : '');

  useEffect(() => setPreviewBroken(false), [icon, version]);

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
      case 'open_url':
        return { type: 'open_url', url: url.trim() };
    }
  }

  const actionValid =
    actionType === 'run_command' ? command.trim() !== '' : url.trim() !== '';

  function handleRemoveFolder() {
    if (
      !window.confirm(
        'Remove this folder? Everything nested inside it will be deleted permanently.',
      )
    ) {
      return;
    }
    run(() => onRemoveFolder(id));
  }

  return (
    <div className="flex w-[min(320px,calc(100vw-32px))] shrink-0 flex-col gap-3.5 self-start rounded-[10px] border border-border bg-bg p-5">
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
        {(icon !== undefined || isFolder) && !previewBroken ? (
          <img
            className="h-full w-full object-cover"
            src={keyImageUrl(path, id, version)}
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
        Icon path or URL
        <input
          type="text"
          className="rounded-sm border border-border bg-bg px-2 py-1.5 font-mono text-sm text-text-h"
          value={draft}
          placeholder="icons/example.png or https://…"
          disabled={busy}
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
          disabled={busy || icon === undefined}
          onClick={() => run(() => onClearIcon(id).then(() => setDraft('')))}
        >
          Clear icon
        </button>
      </div>

      <hr className="w-full border-border" />

      {isFolder ? (
        <>
          <p className="m-0 text-[13px] text-text">This key is a folder.</p>
          <div className="flex gap-2">
            <button
              type="button"
              className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
              disabled={busy}
              onClick={() => onOpenFolder(id)}
            >
              Open folder
            </button>
            <button
              type="button"
              className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
              disabled={busy}
              onClick={handleRemoveFolder}
            >
              Remove folder
            </button>
          </div>
        </>
      ) : (
        <>
          <label className="flex flex-col gap-1.5 text-[13px] text-text">
            Action on press
            <select
              className="rounded-sm border border-border bg-bg px-2 py-1.5 text-sm text-text-h"
              value={actionType}
              disabled={busy}
              onChange={(e) =>
                setActionType(e.target.value as KeyAction['type'])
              }
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

          {actionType === 'open_url' && (
            <label className="flex flex-col gap-1.5 text-[13px] text-text">
              URL
              <input
                type="text"
                className="rounded-sm border border-border bg-bg px-2 py-1.5 font-mono text-sm text-text-h"
                value={url}
                placeholder="https://example.com"
                disabled={busy}
                onChange={(e) => setUrl(e.target.value)}
              />
            </label>
          )}

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
                run(() =>
                  onClearAction(id).then(() => {
                    setCommand('');
                    setUrl('');
                  }),
                )
              }
            >
              Clear action
            </button>
          </div>

          <button
            type="button"
            className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
            disabled={busy}
            onClick={() => run(() => onMakeFolder(id))}
          >
            Make this a folder
          </button>
        </>
      )}

      {error && <div className="text-[13px] text-[#e5484d]">{error}</div>}

      <button
        type="button"
        className="ml-auto cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
        onClick={onClose}
        disabled={busy}
      >
        Close
      </button>
    </div>
  );
}
