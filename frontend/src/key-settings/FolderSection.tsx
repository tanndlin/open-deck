import type { KeySettingsActions } from './types';

interface FolderSectionProps {
  id: number;
  busy: boolean;
  run: (action: () => Promise<void>) => void;
  actions: Pick<KeySettingsActions, 'onOpenFolder' | 'onRemoveFolder'>;
}

export function FolderSection({ id, busy, run, actions }: FolderSectionProps) {
  function handleRemoveFolder() {
    if (
      !window.confirm(
        'Remove this folder? Everything nested inside it will be deleted permanently.',
      )
    ) {
      return;
    }
    run(() => actions.onRemoveFolder(id));
  }

  return (
    <>
      <p className="m-0 text-[13px] text-text">This key is a folder.</p>
      <div className="flex gap-2">
        <button
          type="button"
          className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
          disabled={busy}
          onClick={() => actions.onOpenFolder(id)}
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
  );
}
