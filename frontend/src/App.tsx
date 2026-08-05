import { useEffect, useState } from 'react';
import {
  activatePage,
  clearKeyAction,
  clearKeyIcon,
  createFolder,
  deleteFolder,
  getCurrentPage,
  getKeyCount,
  listKeys,
  setKeyAction,
  setKeyIcon,
  type KeyAction,
  type KeyMap,
  type PagePath,
} from './api';
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
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

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

  async function handleMakeFolder(id: number) {
    await createFolder(path, id);
    await refresh();
  }

  async function handleRemoveFolder(id: number) {
    await deleteFolder(path, id);
    setSelectedKey(null);
    await refresh();
  }

  async function handleOpenFolder(id: number) {
    setSelectedKey(null);
    await navigateTo([...path, id]);
  }

  async function handleGoBack() {
    setSelectedKey(null);
    await navigateTo(path.slice(0, -1));
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
        <h1>Stream Deck Settings</h1>
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

      {keyCount === null ? (
        <p>Loading…</p>
      ) : (
        <div className="mx-auto flex w-full max-w-[70rem] flex-wrap items-start justify-center gap-6">
          <div
            className={`grid max-w-160 flex-1 grid-cols-5 gap-3 ${busy ? 'pointer-events-none opacity-50' : ''}`}
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
              />
            ))}
          </div>

          {selectedKey !== null && (
            <KeySettings
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
