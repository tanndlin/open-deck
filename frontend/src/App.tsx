import { useEffect, useState } from 'react';
import { clearKey, getKeyCount, listKeys, setKey, type KeyMap } from './api';
import { KeySettings } from './KeySettings';
import { KeyTile } from './KeyTile';

function App() {
  const [keyCount, setKeyCount] = useState<number | null>(null);
  const [keys, setKeys] = useState<KeyMap>({});
  const [version, setVersion] = useState(0);
  const [selectedKey, setSelectedKey] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    try {
      const [count, mapping] = await Promise.all([getKeyCount(), listKeys()]);
      setKeyCount(count);
      setKeys(mapping);
      setVersion((v) => v + 1);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  async function handleSet(id: number, path: string) {
    await setKey(id, path);
    await refresh();
  }

  async function handleClear(id: number) {
    await clearKey(id);
    await refresh();
  }

  return (
    <main className="flex min-h-svh flex-col gap-6 p-4 text-center">
      <header className="mb-6">
        <h1>Stream Deck Settings</h1>
      </header>

      {error && (
        <div className="mb-4 rounded-lg border border-accent-border bg-accent-bg px-4 py-3 text-text-h">
          {error}
        </div>
      )}

      {keyCount === null ? (
        <p>Loading…</p>
      ) : (
        <div className="grid max-w-160 grid-cols-5 gap-3 m-auto">
          {Array.from({ length: keyCount }, (_, id) => (
            <KeyTile
              key={id}
              id={id}
              path={keys[id]}
              version={version}
              onClick={setSelectedKey}
            />
          ))}
        </div>
      )}

      {selectedKey !== null && (
        <KeySettings
          id={selectedKey}
          path={keys[selectedKey]}
          version={version}
          onClose={() => setSelectedKey(null)}
          onSet={handleSet}
          onClear={handleClear}
        />
      )}
    </main>
  );
}

export default App;
