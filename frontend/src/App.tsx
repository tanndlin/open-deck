import { useEffect, useRef, useState } from 'react';
import {
  activatePage,
  clearKeyAction,
  clearKeyIcon,
  createFolder,
  deleteFolder,
  getCurrentPage,
  getKeyCount,
  listKeys,
  moveKey,
  setKeyAction,
  setKeyIcon,
  type KeyAction,
  type KeyConfig,
  type KeyMap,
  type PagePath,
} from './api';

function samePagePath(a: PagePath, b: PagePath): boolean {
  return a.length === b.length && a.every((v, i) => v === b[i]);
}

/** How long a key must hover over a folder while being dragged before it opens. */
const FOLDER_HOVER_OPEN_MS = 1000;
import { KeySettings } from './KeySettings';
import { KeyTile } from './KeyTile';
import { PageBar } from './PageBar';

function App() {
  const [keyCount, setKeyCount] = useState<number | null>(null);
  const [path, setPath] = useState<PagePath>([]);
  const [keys, setKeys] = useState<KeyMap>({});
  const [currentDevicePath, setCurrentDevicePath] = useState<PagePath | null>(
    null,
  );
  const [version, setVersion] = useState(0);
  const [selectedKey, setSelectedKey] = useState<number | null>(null);
  const [clipboard, setClipboard] = useState<{
    id: number;
    config: KeyConfig;
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [dragSource, setDragSource] = useState<{
    path: PagePath;
    id: number;
  } | null>(null);
  const hoverTimerRef = useRef<number | null>(null);

  async function refresh(pathOverride?: PagePath) {
    const activePath = pathOverride ?? path;
    try {
      const [count, mapping, devicePath] = await Promise.all([
        getKeyCount(),
        listKeys(activePath),
        getCurrentPage(),
      ]);
      setKeyCount(count);
      setPath(activePath);
      setKeys(mapping);
      setCurrentDevicePath(devicePath);
      setVersion((v) => v + 1);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  useEffect(() => {
    refresh([]);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function navigateTo(newPath: PagePath) {
    setSelectedKey(null);
    // The on-screen page only reflects the persisted config, so it doesn't
    // need to wait on the physical device below — that push is a JPEG
    // encode + USB write per key, and can take a noticeable moment.
    await refresh(newPath);

    setBusy(true);
    try {
      // Best-effort: if this fails, the "device is showing a different
      // page" mismatch in PageBar will surface it.
      await activatePage(newPath);
      setCurrentDevicePath(newPath);
    } catch {
      // ignored — see above
    } finally {
      setBusy(false);
    }
  }

  async function handleSetIcon(id: number, iconPath: string) {
    await setKeyIcon(path, id, iconPath);
    await refresh();
  }

  async function handleClearIcon(id: number) {
    await clearKeyIcon(path, id);
    await refresh();
  }

  async function handleSetAction(id: number, action: KeyAction) {
    await setKeyAction(path, id, action);
    await refresh();
  }

  async function handleClearAction(id: number) {
    await clearKeyAction(path, id);
    await refresh();
  }

  async function handlePaste(id: number, source: KeyConfig) {
    setBusy(true);
    try {
      if (source.icon !== undefined) {
        await setKeyIcon(path, id, source.icon);
      } else {
        await clearKeyIcon(path, id);
      }
      if (source.action !== undefined) {
        await setKeyAction(path, id, source.action);
      } else {
        await clearKeyAction(path, id);
      }
      await refresh();
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }

  async function handleMakeFolder(id: number) {
    await createFolder(path, id);
    await refresh();
  }

  async function handleRemoveFolder(id: number) {
    await deleteFolder(path, id);
    setSelectedKey(null);
    await refresh();
  }

  // Ctrl/Cmd+C copies the selected key's icon + action; Ctrl/Cmd+V pastes
  // them onto whichever key is currently selected; Delete/Backspace clears
  // it (or removes the folder, with confirmation). Ignored while typing in
  // a text field so normal text copy/paste/delete keeps working there.
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      const target = e.target as HTMLElement | null;
      const isEditable =
        target instanceof HTMLInputElement ||
        target instanceof HTMLTextAreaElement ||
        target?.isContentEditable;
      if (isEditable || selectedKey === null) return;

      const key = e.key.toLowerCase();
      const config = keys[selectedKey] ?? { is_folder: false };

      if ((e.ctrlKey || e.metaKey) && key === 'c') {
        setClipboard({ id: selectedKey, config });
      } else if ((e.ctrlKey || e.metaKey) && key === 'v' && clipboard) {
        e.preventDefault();
        handlePaste(selectedKey, clipboard.config);
      } else if (key === 'delete' || key === 'backspace') {
        e.preventDefault();
        if (config.is_folder) {
          if (
            window.confirm(
              'Remove this folder? Everything nested inside it will be deleted permanently.',
            )
          ) {
            handleRemoveFolder(selectedKey);
          }
        } else if (config.icon !== undefined || config.action !== undefined) {
          handlePaste(selectedKey, { is_folder: false });
        }
      }
    }
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selectedKey, keys, clipboard, path]);

  async function handleOpenFolder(id: number) {
    setSelectedKey(null);
    await navigateTo([...path, id]);
  }

  async function handleGoBack() {
    setSelectedKey(null);
    await navigateTo(path.slice(0, -1));
  }

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

  // Dragging a key over a folder and holding it there opens the folder, so
  // the drag can continue into its subpage without letting go first.
  function handleHoverFolder(id: number) {
    clearHoverTimer();
    hoverTimerRef.current = window.setTimeout(() => {
      hoverTimerRef.current = null;
      handleOpenFolder(id);
    }, FOLDER_HOVER_OPEN_MS);
  }

  // Dragging a key over the back arrow and holding it there navigates up a
  // level, same idea as hovering a folder open.
  function handleHoverBack() {
    clearHoverTimer();
    hoverTimerRef.current = window.setTimeout(() => {
      hoverTimerRef.current = null;
      handleGoBack();
    }, FOLDER_HOVER_OPEN_MS);
  }

  function handleHoverCancel() {
    clearHoverTimer();
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
      // Second click on an already-selected key triggers its action.
      // Key 0 on a subpage is reserved to go up a level, regardless of
      // whatever's configured there. Otherwise, for now, only folders do
      // anything here.
      if (id === 0 && path.length > 0) {
        handleGoBack();
        return;
      }
      const config = keys[id] ?? { is_folder: false };
      if (config.is_folder) {
        handleOpenFolder(id);
      }
      return;
    }
    setSelectedKey(id);
  }

  return (
    <main className="flex min-h-svh flex-col gap-6 p-4 text-center">
      <header className="mb-2">
        <h1>Open Deck Settings</h1>
      </header>

      {error && (
        <div className="mb-4 rounded-lg border border-accent-border bg-accent-bg px-4 py-3 text-text-h">
          {error}
        </div>
      )}

      <PageBar
        path={path}
        currentDevicePath={currentDevicePath}
        onNavigate={navigateTo}
      />

      {clipboard && (
        <p className="-mt-4 text-xs text-text opacity-70">
          Key {clipboard.id} copied — select another key and press Ctrl+V to
          paste
        </p>
      )}

      {keyCount === null ? (
        <p>Loading…</p>
      ) : (
        <div className="mx-auto flex w-full max-w-[70rem] flex-wrap items-start justify-center gap-6">
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
                onClick={handleTileClick}
                onDragStart={handleDragStart}
                onDragEnd={handleDragEnd}
                onDropKey={handleDropKey}
                onHoverFolder={handleHoverFolder}
                onHoverBack={handleHoverBack}
                onHoverCancel={handleHoverCancel}
              />
            ))}
          </div>

          {selectedKey !== null && (
            <KeySettings
              key={selectedKey}
              id={selectedKey}
              path={path}
              config={keys[selectedKey] ?? { is_folder: false }}
              version={version}
              onClose={() => setSelectedKey(null)}
              onSetIcon={handleSetIcon}
              onClearIcon={handleClearIcon}
              onSetAction={handleSetAction}
              onClearAction={handleClearAction}
              onMakeFolder={handleMakeFolder}
              onRemoveFolder={handleRemoveFolder}
              onOpenFolder={handleOpenFolder}
            />
          )}
        </div>
      )}
    </main>
  );
}

export default App;
