import { useState } from 'react';
import { ActionTextInput } from './ActionTextInput';
import { SetActionButton } from './SetActionButton';
import type { ActionFieldProps } from './types';

/** Splits a `+`-joined hotkey string (e.g. "ctrl+alt+del") into key names. */
function parseHotkeyKeys(input: string): string[] {
  return input
    .split('+')
    .map((k) => k.trim())
    .filter((k) => k !== '');
}

export function HotkeyField({ action, disabled, onSubmit }: ActionFieldProps) {
  const [hotkeyInput, setHotkeyInput] = useState(
    action?.type === 'hotkey' ? action.keys.join('+') : '',
  );
  const keys = parseHotkeyKeys(hotkeyInput);
  const valid = keys.length > 0;

  function submit() {
    if (valid) onSubmit({ type: 'hotkey', keys });
  }

  return (
    <>
      <ActionTextInput
        label="Keys (joined with +)"
        placeholder="ctrl+c, escape, del"
        value={hotkeyInput}
        onChange={setHotkeyInput}
        disabled={disabled}
        onSubmit={submit}
      />
      <SetActionButton disabled={disabled || !valid} onClick={submit} />
    </>
  );
}
