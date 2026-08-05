import { useEffect, useState } from 'react';
import { HotkeyField } from './action-fields/HotkeyField';
import { OpenFolderField } from './action-fields/OpenFolderField';
import { OpenUrlField } from './action-fields/OpenUrlField';
import { RunCommandField } from './action-fields/RunCommandField';
import { TypeTextField } from './action-fields/TypeTextField';
import { keyImageUrl } from './api';
import {
  ACTION_TYPES,
  type ActionType,
  type KeyAction,
  type KeyConfig,
  type PagePath,
} from './types';

interface KeySettingsProps {
  id: number;
  path: PagePath;
  config: KeyConfig;
  version: number;
  onClose: () => void;
  onSetIcon: (id: number, path: string) => Promise<void>;
  onClearIcon: (id: number) => Promise<void>;
  onSetTitle: (id: number, title: string) => Promise<void>;
  onClearTitle: (id: number) => Promise<void>;
  onSetAction: (id: number, action: KeyAction) => Promise<void>;
  onClearAction: (id: number) => Promise<void>;
  onMakeFolder: (id: number) => Promise<void>;
  onRemoveFolder: (id: number) => Promise<void>;
  onOpenFolder: (id: number) => void;
}

export function KeySettings({
  id,
  path,
  config,
  version,
  onClose,
  onSetIcon,
  onClearIcon,
  onSetTitle,
  onClearTitle,
  onSetAction,
  onClearAction,
  onMakeFolder,
  onRemoveFolder,
  onOpenFolder,
}: KeySettingsProps) {
  const { icon, title, action, is_folder: isFolder } = config;
  const [draft, setDraft] = useState(icon ?? '');
  const [titleDraft, setTitleDraft] = useState(title ?? '');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [previewBroken, setPreviewBroken] = useState(false);

  const [actionType, setActionType] = useState<ActionType>(
    action?.type ?? 'run_command',
  );
  // Bumped after clearing the action to force the currently selected
  // ActionTypeField to remount and reset its own (now-owned) input state.
  const [clearNonce, setClearNonce] = useState(0);

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
            className="h-full w-full object-contain"
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
          onKeyDown={(e) => {
            if (e.key === 'Enter' && draft.trim() !== '') {
              e.preventDefault();
              run(() => onSetIcon(id, draft.trim()));
            }
          }}
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

      <label className="flex flex-col gap-1.5 text-[13px] text-text">
        Title
        <input
          type="text"
          className="rounded-sm border border-border bg-bg px-2 py-1.5 text-sm text-text-h"
          value={titleDraft}
          placeholder="Shown on the key"
          disabled={busy}
          onChange={(e) => setTitleDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Enter' && titleDraft.trim() !== '') {
              e.preventDefault();
              run(() => onSetTitle(id, titleDraft.trim()));
            }
          }}
        />
      </label>

      <div className="flex gap-2">
        <button
          type="button"
          className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
          disabled={busy || titleDraft.trim() === ''}
          onClick={() => run(() => onSetTitle(id, titleDraft.trim()))}
        >
          Set title
        </button>
        <button
          type="button"
          className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
          disabled={busy || title === undefined}
          onClick={() =>
            run(() => onClearTitle(id).then(() => setTitleDraft('')))
          }
        >
          Clear title
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
              run(() =>
                onClearAction(id).then(() => setClearNonce((n) => n + 1)),
              )
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

interface ActionTypeFieldProps {
  actionType: ActionType;
  action: KeyAction | undefined;
  disabled: boolean;
  onSubmit: (action: KeyAction) => void;
}

function ActionTypeField({
  actionType,
  action,
  disabled,
  onSubmit,
}: ActionTypeFieldProps) {
  switch (actionType) {
    case 'run_command':
      return (
        <RunCommandField
          action={action}
          disabled={disabled}
          onSubmit={onSubmit}
        />
      );
    case 'open_url':
      return (
        <OpenUrlField action={action} disabled={disabled} onSubmit={onSubmit} />
      );
    case 'open_folder':
      return (
        <OpenFolderField
          action={action}
          disabled={disabled}
          onSubmit={onSubmit}
        />
      );
    case 'type_text':
      return (
        <TypeTextField
          action={action}
          disabled={disabled}
          onSubmit={onSubmit}
        />
      );
    case 'hotkey':
      return (
        <HotkeyField action={action} disabled={disabled} onSubmit={onSubmit} />
      );
  }
}
