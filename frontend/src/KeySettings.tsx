import { useEffect, useState } from 'react';
import { ActionSection } from './key-settings/ActionSection';
import { FolderSection } from './key-settings/FolderSection';
import { IconSection } from './key-settings/IconSection';
import { TitleSection } from './key-settings/TitleSection';
import type { KeyAction, KeyConfig, PagePath } from './types';

interface KeySettingsProps {
  id: number;
  path: PagePath;
  config: KeyConfig;
  version: number;
  onClose: () => void;
  onSetIcon: (id: number, iconPath: string) => Promise<void>;
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
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose();
    }
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [onClose]);

  async function run(fn: () => Promise<void>) {
    setBusy(true);
    setError(null);
    try {
      await fn();
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
        onSetIcon={onSetIcon}
        onClearIcon={onClearIcon}
      />

      <TitleSection
        id={id}
        title={title}
        busy={busy}
        run={run}
        onSetTitle={onSetTitle}
        onClearTitle={onClearTitle}
      />

      <hr className="w-full border-border" />

      {isFolder ? (
        <FolderSection
          id={id}
          busy={busy}
          run={run}
          onOpenFolder={onOpenFolder}
          onRemoveFolder={onRemoveFolder}
        />
      ) : (
        <ActionSection
          id={id}
          action={action}
          busy={busy}
          run={run}
          onSetAction={onSetAction}
          onClearAction={onClearAction}
          onMakeFolder={onMakeFolder}
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
