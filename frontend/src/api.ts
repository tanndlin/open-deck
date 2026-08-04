export type KeyMap = Record<string, string>;

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

export async function setKey(id: number, path: string): Promise<void> {
  await checkOk(
    await fetch(`/api/keys/${id}`, {
      method: 'PUT',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ path }),
    }),
  );
}

export async function clearKey(id: number): Promise<void> {
  await checkOk(await fetch(`/api/keys/${id}`, { method: 'DELETE' }));
}
