import { useEffect, useState } from 'react';
import { keyImageUrl } from '../api';
import type { PagePath } from '../types';

interface IconSectionProps {
  id: number;
  path: PagePath;
  icon: string | undefined;
  isFolder: boolean;
  version: number;
  busy: boolean;
  run: (fn: () => Promise<void>) => void;
  onSetIcon: (id: number, path: string) => Promise<void>;
  onClearIcon: (id: number) => Promise<void>;
}

export function IconSection({
  id,
  path,
  icon,
  isFolder,
  version,
  busy,
  run,
  onSetIcon,
  onClearIcon,
}: IconSectionProps) {
  const [draft, setDraft] = useState(icon ?? '');
  const [previewBroken, setPreviewBroken] = useState(false);

  useEffect(() => setPreviewBroken(false), [icon, version]);

  return (
    <>
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
    </>
  );
}
