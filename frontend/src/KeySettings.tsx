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

export function KeySettings({ id, path, version, onClose, onSet, onClear }: KeySettingsProps) {
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
    <div className="modal-backdrop" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Key {id}</h2>
          <button type="button" className="modal-close" onClick={onClose} aria-label="Close">
            ×
          </button>
        </div>

        <div className="modal-preview">
          {path !== undefined && !previewBroken ? (
            <img
              src={keyImageUrl(id, version)}
              alt={`Key ${id}`}
              onError={() => setPreviewBroken(true)}
            />
          ) : (
            <span className="modal-preview-empty">No icon set</span>
          )}
        </div>

        <label className="modal-field">
          Icon path
          <input
            type="text"
            value={draft}
            placeholder="icons/example.png"
            disabled={busy}
            autoFocus
            onChange={(e) => setDraft(e.target.value)}
          />
        </label>

        {error && <div className="key-tile-error">{error}</div>}

        <div className="modal-actions">
          <button
            type="button"
            disabled={busy || draft.trim() === ''}
            onClick={() => run(() => onSet(id, draft.trim()))}
          >
            Set
          </button>
          <button
            type="button"
            disabled={busy || path === undefined}
            onClick={() => run(() => onClear(id).then(() => setDraft('')))}
          >
            Clear
          </button>
          <button type="button" onClick={onClose} disabled={busy}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
