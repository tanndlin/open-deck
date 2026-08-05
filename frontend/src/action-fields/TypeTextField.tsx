import { useState } from 'react';
import { ActionTextInput } from './ActionTextInput';
import { SetActionButton } from './SetActionButton';
import type { ActionFieldProps } from './types';

export function TypeTextField({
  action,
  disabled,
  onSubmit,
}: ActionFieldProps) {
  const [text, setText] = useState(
    action?.type === 'type_text' ? action.text : '',
  );
  const valid = text !== '';

  function submit() {
    if (valid) onSubmit({ type: 'type_text', text });
  }

  return (
    <>
      <ActionTextInput
        label="Text to type"
        placeholder="Hello, world!"
        value={text}
        onChange={setText}
        disabled={disabled}
        onSubmit={submit}
      />
      <SetActionButton disabled={disabled || !valid} onClick={submit} />
    </>
  );
}
