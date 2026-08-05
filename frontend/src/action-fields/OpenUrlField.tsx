import { useState } from 'react';
import { ActionTextInput } from './ActionTextInput';
import { SetActionButton } from './SetActionButton';
import type { ActionFieldProps } from './types';

export function OpenUrlField({ action, disabled, onSubmit }: ActionFieldProps) {
  const [url, setUrl] = useState(action?.type === 'open_url' ? action.url : '');
  const valid = url.trim() !== '';

  function submit() {
    if (valid) onSubmit({ type: 'open_url', url: url.trim() });
  }

  return (
    <>
      <ActionTextInput
        label="URL"
        placeholder="https://example.com"
        value={url}
        onChange={setUrl}
        disabled={disabled}
        onSubmit={submit}
      />
      <SetActionButton disabled={disabled || !valid} onClick={submit} />
    </>
  );
}
