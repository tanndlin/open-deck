export interface RunCommandAction {
  type: 'run_command';
  command: string;
}

/** Union of all action kinds. Add new variants here as the backend gains them. */
export type KeyAction = RunCommandAction;

export interface KeyConfig {
  icon?: string;
  action?: KeyAction;
}

export type KeyMap = Record<string, KeyConfig>;

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

export async function listKeys(): Promise<KeyMap> {
  const res = await checkOk(await fetch('/api/keys'));
  return res.json();
}

export async function setKeyIcon(id: number, path: string): Promise<void> {
  await checkOk(
    await fetch(`/api/keys/${id}/icon`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path }),
    }),
  );
}

export async function clearKeyIcon(id: number): Promise<void> {
  await checkOk(await fetch(`/api/keys/${id}/icon`, { method: 'DELETE' }));
}

export async function setKeyAction(id: number, action: KeyAction): Promise<void> {
  await checkOk(
    await fetch(`/api/keys/${id}/action`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(action),
    }),
  );
}

export async function clearKeyAction(id: number): Promise<void> {
  await checkOk(await fetch(`/api/keys/${id}/action`, { method: 'DELETE' }));
}

/** URL for the image currently pushed to a key. `version` busts the browser cache after updates. */
export function keyImageUrl(id: number, version: number): string {
  return `/api/keys/${id}/image?v=${version}`;
}
