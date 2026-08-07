import { useRef, useState } from 'react';
import { moveKey, uploadKeyIcon } from './api';
import { KeyTile } from './KeyTile';
import type { KeyMap, PagePath } from './types';

/** How long a key must hover over a folder while being dragged before it opens. */
const FOLDER_HOVER_OPEN_MS = 1000;

function samePagePath(a: PagePath, b: PagePath): boolean {
  return a.length === b.length && a.every((v, i) => v === b[i]);
}

interface KeyGridProps {
  keyCount: number;
  path: PagePath;
  keys: KeyMap;
  version: number;
  selectedKey: number | null;
  /** The physically-pressed key, briefly, for a visual flash. */
  flashedKey: number | null;
  busy: boolean;
  setBusy: (busy: boolean) => void;
  setError: (error: string | null) => void;
  refresh: () => Promise<void>;
  onSelectKey: (id: number) => void;
  onOpenFolder: (id: number) => Promise<void>;
  onGoBack: () => Promise<void>;
}

export function KeyGrid({
  keyCount,
  path,
  keys,
  version,
  selectedKey,
  flashedKey,
  busy,
  setBusy,
  setError,
  refresh,
  onSelectKey,
  onOpenFolder,
  onGoBack,
}: KeyGridProps) {
  const [dragSource, setDragSource] = useState<{
    path: PagePath;
    id: number;
  } | null>(null);
  const hoverTimerRef = useRef<number | null>(null);

  function clearHoverTimer() {
    if (hoverTimerRef.current !== null) {
      window.clearTimeout(hoverTimerRef.current);
      hoverTimerRef.current = null;
    }
  }

  function handleDragStart(id: number) {
    setDragSource({ path, id });
  }

  function handleDragEnd() {
    clearHoverTimer();
    setDragSource(null);
  }

  // Holding a dragged key over a folder opens it, so the drag can continue
  // into the subpage without letting go first.
  function handleHoverFolder(id: number) {
    clearHoverTimer();
    hoverTimerRef.current = window.setTimeout(() => {
      hoverTimerRef.current = null;
      onOpenFolder(id);
    }, FOLDER_HOVER_OPEN_MS);
  }

  // Same idea as handleHoverFolder, but navigates up a level.
  function handleHoverBack() {
    clearHoverTimer();
    hoverTimerRef.current = window.setTimeout(() => {
      hoverTimerRef.current = null;
      onGoBack();
    }, FOLDER_HOVER_OPEN_MS);
  }

  function handleHoverCancel() {
    clearHoverTimer();
  }

  async function handleDropFile(id: number, file: File) {
    setBusy(true);
    try {
      await uploadKeyIcon(path, id, file);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleDropKey(id: number) {
    const source = dragSource;
    setDragSource(null);
    clearHoverTimer();
    if (!source) return;
    if (samePagePath(source.path, path) && source.id === id) return;

    setBusy(true);
    try {
      await moveKey(source.path, source.id, path, id);
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  function handleTileClick(id: number) {
    if (selectedKey === id) {
      // Second click triggers the key's action. Key 0 on a subpage always
      // goes back; otherwise only folders do anything here.
      if (id === 0 && path.length > 0) {
        onGoBack();
        return;
      }
      const config = keys[id] ?? { is_folder: false };
      if (config.is_folder) {
        onOpenFolder(id);
      }
      return;
    }
    onSelectKey(id);
  }

  return (
    <div
      className={`grid max-w-160 flex-1 grid-cols-5 gap-3 ${busy && !dragSource ? 'pointer-events-none opacity-50' : ''}`}
    >
      {Array.from({ length: keyCount }, (_, id) => (
        <KeyTile
          key={id}
          id={id}
          path={path}
          config={keys[id] ?? { is_folder: false }}
          version={version}
          selected={selectedKey === id}
          flashed={flashedKey === id}
          onClick={handleTileClick}
          drag={{
            onDragStart: handleDragStart,
            onDragEnd: handleDragEnd,
            onDropKey: handleDropKey,
            onDropFile: handleDropFile,
            onHoverFolder: handleHoverFolder,
            onHoverBack: handleHoverBack,
            onHoverCancel: handleHoverCancel,
          }}
        />
      ))}
    </div>
  );
}
