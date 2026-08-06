import { useEffect, useState } from 'react';
import { ActionSection } from './key-settings/ActionSection';
import { FolderSection } from './key-settings/FolderSection';
import { IconSection } from './key-settings/IconSection';
import { TitleSection } from './key-settings/TitleSection';
import type { KeySettingsActions } from './key-settings/types';
import type { KeyConfig, PagePath } from './types';

interface KeySettingsProps {
  id: number;
  path: PagePath;
  config: KeyConfig;
  version: number;
  onClose: () => void;
  actions: KeySettingsActions;
}

export function KeySettings({
  id,
  path,
  config,
  version,
  onClose,
  actions,
}: KeySettingsProps) {
  const { icon, title, action, is_folder: isFolder } = config;
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

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
    <div className="flex w-[320px] mr-auto shrink-0 flex-col gap-3.5 self-start rounded-[10px] border border-border bg-bg p-5">
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

      <IconSection
        id={id}
        path={path}
        icon={icon}
        isFolder={isFolder}
        version={version}
        busy={busy}
        run={run}
        actions={actions}
      />

      <TitleSection
        id={id}
        title={title}
        busy={busy}
        run={run}
        actions={actions}
      />

      <hr className="w-full border-border" />

      {isFolder ? (
        <FolderSection id={id} busy={busy} run={run} actions={actions} />
      ) : (
        <ActionSection
          id={id}
          action={action}
          busy={busy}
          run={run}
          actions={actions}
        />
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
