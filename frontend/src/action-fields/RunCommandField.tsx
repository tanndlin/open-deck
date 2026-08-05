import { useState } from 'react';
import { ActionTextInput } from './ActionTextInput';
import { SetActionButton } from './SetActionButton';
import type { ActionFieldProps } from './types';

export function RunCommandField({
  action,
  disabled,
  onSubmit,
}: ActionFieldProps) {
  const [command, setCommand] = useState(
    action?.type === 'run_command' ? action.command : '',
  );
  const valid = command.trim() !== '';

  function submit() {
    if (valid) onSubmit({ type: 'run_command', command: command.trim() });
  }

  return (
    <>
      <ActionTextInput
        label="Command"
        placeholder="python script.py"
        value={command}
        onChange={setCommand}
        disabled={disabled}
        onSubmit={submit}
      />
      <SetActionButton disabled={disabled || !valid} onClick={submit} />
    </>
  );
}
