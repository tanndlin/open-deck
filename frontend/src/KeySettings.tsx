import { useEffect, useState } from 'react';
import { keyImageUrl } from './api';

interface KeySettingsProps {
  id: number;
  path?: string;
  version: number;
  onClose: () => void;
  onSet: (id: number, path: string) => Promise<void>;
  onClear: (id: number) => Promise<void>;
}

export function KeySettings({
  id,
  path,
  version,
  onClose,
  onSet,
  onClear,
}: KeySettingsProps) {
  const [draft, setDraft] = useState(path ?? '');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [previewBroken, setPreviewBroken] = useState(false);

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

  return (
    <div
      className="fixed inset-0 z-10 flex items-center justify-center bg-black/50"
      onClick={onClose}
    >
      <div
        className="flex w-[min(320px,calc(100vw-32px))] flex-col gap-3.5 rounded-[10px] border border-border bg-bg p-5 shadow-modal"
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

        {error && <div className="text-[13px] text-[#e5484d]">{error}</div>}

        <div className="flex gap-2">
          <button
            type="button"
            className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
            disabled={busy || draft.trim() === ''}
            onClick={() => run(() => onSet(id, draft.trim()))}
          >
            Set
          </button>
          <button
            type="button"
            className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
            disabled={busy || path === undefined}
            onClick={() => run(() => onClear(id).then(() => setDraft('')))}
          >
            Clear
          </button>
          <button
            type="button"
            className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
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
