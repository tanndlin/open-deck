import { useEffect, useState } from 'react';
import {
  activatePage,
  clearKeyAction,
  clearKeyIcon,
  clearKeyTitle,
  createFolder,
  deleteFolder,
  getCurrentPage,
  getKeyCount,
  listKeys,
  setKeyAction,
  setKeyIcon,
  setKeyTitle,
} from './api';
import { KeyGrid } from './KeyGrid';
import { KeySettings } from './KeySettings';
import { PageBar } from './PageBar';
import type { KeyAction, KeyConfig, KeyMap, PagePath } from './types';

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
    // Doesn't need to wait on the device push below (a JPEG encode + USB
    // write per key), since the on-screen page only reflects the config.
    await refresh(newPath);

    setBusy(true);
    try {
      // Best-effort: failures surface via PageBar's device-mismatch notice.
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

  async function handleSetTitle(id: number, title: string) {
    await setKeyTitle(path, id, title);
    await refresh();
  }

  async function handleClearTitle(id: number) {
    await clearKeyTitle(path, id);
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
      if (source.title !== undefined) {
        await setKeyTitle(path, id, source.title);
      } else {
        await clearKeyTitle(path, id);
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

  // Ctrl/Cmd+C/V copy/paste a key's icon+action; Delete/Backspace clears it.
  // Ignored while typing in a text field.
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
        } else if (
          config.icon !== undefined ||
          config.title !== undefined ||
          config.action !== undefined
        ) {
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
        <div
          id="home-grid"
          className="mx-auto flex w-full flex-wrap items-start justify-center gap-6"
        >
          <div />
          <KeyGrid
            keyCount={keyCount}
            path={path}
            keys={keys}
            version={version}
            selectedKey={selectedKey}
            busy={busy}
            setBusy={setBusy}
            setError={setError}
            refresh={refresh}
            onSelectKey={setSelectedKey}
            onOpenFolder={handleOpenFolder}
            onGoBack={handleGoBack}
          />

          {selectedKey !== null && (
            <KeySettings
              key={selectedKey}
              id={selectedKey}
              path={path}
              config={keys[selectedKey] ?? { is_folder: false }}
              version={version}
              onClose={() => setSelectedKey(null)}
              actions={{
                onSetIcon: handleSetIcon,
                onClearIcon: handleClearIcon,
                onSetTitle: handleSetTitle,
                onClearTitle: handleClearTitle,
                onSetAction: handleSetAction,
                onClearAction: handleClearAction,
                onMakeFolder: handleMakeFolder,
                onRemoveFolder: handleRemoveFolder,
                onOpenFolder: handleOpenFolder,
              }}
            />
          )}
        </div>
      )}
    </main>
  );
}

export default App;
