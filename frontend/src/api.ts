interface ActionPayloads {
  run_command: { command: string };
  open_url: { url: string };
  open_folder: { path: string };
  type_text: { text: string };
  hotkey: { keys: string[] };
}

export type KeyAction = {
  [K in keyof ActionPayloads]: { type: K } & ActionPayloads[K];
}[keyof ActionPayloads];

export type ActionType = keyof ActionPayloads;

// still need RunCommandAction etc. elsewhere? derive them instead of hand-writing:
export type RunCommandAction = Extract<KeyAction, { type: 'run_command' }>;

const ACTION_LABELS = {
  run_command: 'Run command',
  open_url: 'Open webpage',
  open_folder: 'Open folder',
  type_text: 'Type text',
  hotkey: 'Hotkey',
} satisfies Record<ActionType, string>;

export const ACTION_TYPES = Object.entries(ACTION_LABELS).map(
  ([value, label]) => ({
    value: value as ActionType,
    label,
  }),
);

export interface KeyConfig {
  icon?: string;
  action?: KeyAction;
  is_folder: boolean;
}

export type KeyMap = Record<string, KeyConfig>;

/** A sequence of key indices followed from the home page. Empty is home. */
export type PagePath = number[];

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
