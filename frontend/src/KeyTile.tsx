import { useState } from 'react';

interface KeyTileProps {
  id: number;
  path?: string;
  onSet: (id: number, path: string) => Promise<void>;
  onClear: (id: number) => Promise<void>;
}

export function KeyTile({ id, path, onSet, onClear }: KeyTileProps) {
  const [draft, setDraft] = useState(path ?? '');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
    <div className="key-tile">
      <div className="key-tile-header">Key {id}</div>
      <input
        type="text"
        value={draft}
        placeholder="icons/example.png"
        disabled={busy}
        onChange={(e) => setDraft(e.target.value)}
      />
      <div className="key-tile-actions">
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
      </div>
      {error && <div className="key-tile-error">{error}</div>}
    </div>
  );
}
