import { useState } from 'react';
import type { KeySettingsActions } from './types';

interface TitleSectionProps {
  id: number;
  title: string | undefined;
  busy: boolean;
  run: (action: () => Promise<void>) => void;
  actions: Pick<KeySettingsActions, 'onSetTitle' | 'onClearTitle'>;
}

export function TitleSection({
  id,
  title,
  busy,
  run,
  actions,
}: TitleSectionProps) {
  const [titleDraft, setTitleDraft] = useState(title ?? '');

  return (
    <>
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
              run(() => actions.onSetTitle(id, titleDraft.trim()));
            }
          }}
        />
      </label>

      <div className="flex gap-2">
        <button
          type="button"
          className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
          disabled={busy || titleDraft.trim() === ''}
          onClick={() => run(() => actions.onSetTitle(id, titleDraft.trim()))}
        >
          Set title
        </button>
        <button
          type="button"
          className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
          disabled={busy || title === undefined}
          onClick={() =>
            run(() => actions.onClearTitle(id).then(() => setTitleDraft('')))
          }
        >
          Clear title
        </button>
      </div>
    </>
  );
}
