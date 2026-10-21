import { useState } from 'react';
import { ActionTextInput } from './ActionTextInput';
import { SetActionButton } from './SetActionButton';
import type { ActionFieldProps } from './types';

const METHODS = ['GET', 'POST', 'PUT', 'PATCH', 'DELETE'];

/** Parses one `Name: value` header per line, skipping lines without a name. */
function parseHeaders(input: string): { name: string; value: string }[] {
  return input.split('\n').flatMap((line) => {
    const colon = line.indexOf(':');
    if (colon === -1) return [];
    const name = line.slice(0, colon).trim();
    const value = line.slice(colon + 1).trim();
    return name === '' ? [] : [{ name, value }];
  });
}

const textareaClass =
  'min-h-16 rounded-sm border border-border bg-bg px-2 py-1.5 font-mono text-sm text-text-h';

export function HttpRequestField({
  action,
  disabled,
  onSubmit,
}: ActionFieldProps) {
  const current = action?.type === 'http_request' ? action : undefined;
  const [method, setMethod] = useState(current?.method ?? 'GET');
  const [url, setUrl] = useState(current?.url ?? '');
  const [headersInput, setHeadersInput] = useState(
    current?.headers.map((h) => `${h.name}: ${h.value}`).join('\n') ?? '',
  );
  const [body, setBody] = useState(current?.body ?? '');
  const valid = url.trim() !== '';

  function submit() {
    if (valid)
      onSubmit({
        type: 'http_request',
        method,
        url: url.trim(),
        headers: parseHeaders(headersInput),
        body,
      });
  }

  return (
    <>
      <label className="flex flex-col gap-1.5 text-[13px] text-text">
        Method
        <select
          className="rounded-sm border border-border bg-bg px-2 py-1.5 text-sm text-text-h"
          value={method}
          disabled={disabled}
          onChange={(e) => setMethod(e.target.value)}
        >
          {METHODS.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
      </label>
      <ActionTextInput
        label="URL"
        placeholder="https://example.com/webhook"
        value={url}
        onChange={setUrl}
        disabled={disabled}
        onSubmit={submit}
      />
      <label className="flex flex-col gap-1.5 text-[13px] text-text">
        Headers (one per line)
        <textarea
          className={textareaClass}
          placeholder="Content-Type: application/json"
          value={headersInput}
          disabled={disabled}
          onChange={(e) => setHeadersInput(e.target.value)}
        />
      </label>
      <label className="flex flex-col gap-1.5 text-[13px] text-text">
        Body
        <textarea
          className={textareaClass}
          placeholder='{"key": "value"}'
          value={body}
          disabled={disabled}
          onChange={(e) => setBody(e.target.value)}
        />
      </label>
      <SetActionButton disabled={disabled || !valid} onClick={submit} />
    </>
  );
}
