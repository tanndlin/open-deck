import type { KeyAction, KeyMap, PagePath } from './types';

export function formatPagePath(path: PagePath): string {
  return path.length === 0 ? 'home' : path.join('.');
}

/** Parses the `.`-joined path string the server sends back (or `"home"`). */
export function parsePagePath(raw: string): PagePath {
  return raw === 'home' ? [] : raw.split('.').map(Number);
}

async function checkOk(res: Response): Promise<Response> {
  if (!res.ok) {
    const text = await res.text();
    throw new Error(text || `${res.status} ${res.statusText}`);
  }
  return res;
}

export async function getKeyCount(): Promise<number> {
  const res = await checkOk(await fetch('/api/key-count'));
  return res.json();
}

/** The page path currently pushed onto the physical device. */
export async function getCurrentPage(): Promise<PagePath> {
  const res = await checkOk(await fetch('/api/current-page'));
  const { path }: { path: string } = await res.json();
  return parsePagePath(path);
}

/** Pushes the page at `path` onto the physical device right away, without waiting for a key press. */
export async function activatePage(path: PagePath): Promise<void> {
  await checkOk(
    await fetch(`/api/pages/${formatPagePath(path)}/activate`, {
      method: 'POST',
    }),
  );
}

export async function listKeys(path: PagePath): Promise<KeyMap> {
  const res = await checkOk(
    await fetch(`/api/pages/${formatPagePath(path)}/keys`),
  );
  return res.json();
}

export async function setKeyIcon(
  path: PagePath,
  id: number,
  iconPath: string,
): Promise<void> {
  await checkOk(
    await fetch(`/api/pages/${formatPagePath(path)}/keys/${id}/icon`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path: iconPath }),
    }),
  );
}

/** Uploads a dropped file's bytes as key `id`'s icon (browsers don't expose a dragged file's real path). */
export async function uploadKeyIcon(
  path: PagePath,
  id: number,
  file: File,
): Promise<void> {
  await checkOk(
    await fetch(
      `/api/pages/${formatPagePath(path)}/keys/${id}/icon/upload?filename=${encodeURIComponent(
        file.name,
      )}`,
      {
        method: 'PUT',
        headers: { 'Content-Type': 'application/octet-stream' },
        body: file,
      },
    ),
  );
}

export async function clearKeyIcon(path: PagePath, id: number): Promise<void> {
  await checkOk(
    await fetch(`/api/pages/${formatPagePath(path)}/keys/${id}/icon`, {
      method: 'DELETE',
    }),
  );
}

export async function setKeyTitle(
  path: PagePath,
  id: number,
  title: string,
): Promise<void> {
  await checkOk(
    await fetch(`/api/pages/${formatPagePath(path)}/keys/${id}/title`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ title }),
    }),
  );
}

export async function clearKeyTitle(path: PagePath, id: number): Promise<void> {
  await checkOk(
    await fetch(`/api/pages/${formatPagePath(path)}/keys/${id}/title`, {
      method: 'DELETE',
    }),
  );
}

export async function setKeyAction(
  path: PagePath,
  id: number,
  action: KeyAction,
): Promise<void> {
  await checkOk(
    await fetch(`/api/pages/${formatPagePath(path)}/keys/${id}/action`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(action),
    }),
  );
}

export async function clearKeyAction(
  path: PagePath,
  id: number,
): Promise<void> {
  await checkOk(
    await fetch(`/api/pages/${formatPagePath(path)}/keys/${id}/action`, {
      method: 'DELETE',
    }),
  );
}

export async function createFolder(path: PagePath, id: number): Promise<void> {
  await checkOk(
    await fetch(`/api/pages/${formatPagePath(path)}/keys/${id}/folder`, {
      method: 'PUT',
    }),
  );
}

/** Deletes everything nested inside key `id`'s folder along with it. */
export async function deleteFolder(path: PagePath, id: number): Promise<void> {
  await checkOk(
    await fetch(`/api/pages/${formatPagePath(path)}/keys/${id}/folder`, {
      method: 'DELETE',
    }),
  );
}

/** Moves a key's config to another slot (possibly on a different page); swaps if the destination is occupied. */
export async function moveKey(
  fromPath: PagePath,
  fromId: number,
  toPath: PagePath,
  toId: number,
): Promise<void> {
  await checkOk(
    await fetch('/api/move', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        from_path: formatPagePath(fromPath),
        from_id: fromId,
        to_path: formatPagePath(toPath),
        to_id: toId,
      }),
    }),
  );
}

/** URL for the image currently pushed to a key on `path`. `version` busts the browser cache after updates. */
export function keyImageUrl(
  path: PagePath,
  id: number,
  version: number,
): string {
  return `/api/pages/${formatPagePath(path)}/keys/${id}/image?v=${version}`;
}

/** Mirrors the server's `ServerEvent` (see `src/api.rs`). `path` is a formatted `PagePath` (see `formatPagePath`). */
export type DeviceEvent =
  | { type: 'page_changed'; path: string }
  | { type: 'key_pressed'; path: string; id: number };

/**
 * Subscribes to `/api/ws` for live device state, so the GUI's notion of the
 * current page (and physical key presses) never drifts from what the device
 * is actually doing. Reconnects automatically if the connection drops.
 * Returns a function that tears the subscription down.
 */
export function subscribeDeviceEvents(
  onEvent: (event: DeviceEvent) => void,
): () => void {
  let socket: WebSocket | null = null;
  let stopped = false;
  let retryTimer: number | null = null;

  function connect() {
    if (stopped) return;
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:';
    socket = new WebSocket(`${protocol}//${location.host}/api/ws`);
    socket.onmessage = (e) => {
      onEvent(JSON.parse(e.data as string) as DeviceEvent);
    };
    // Fires on a clean close, a dropped connection, or a failed handshake
    // (which also triggers 'error' immediately before this).
    socket.onclose = () => {
      if (stopped) return;
      retryTimer = window.setTimeout(connect, 1000);
    };
  }
  connect();

  return () => {
    stopped = true;
    if (retryTimer !== null) window.clearTimeout(retryTimer);
    socket?.close();
  };
}
