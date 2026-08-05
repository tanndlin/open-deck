import { useState } from 'react';
import { ActionTextInput } from './ActionTextInput';
import { SetActionButton } from './SetActionButton';
import type { ActionFieldProps } from './types';

export function OpenFolderField({
  action,
  disabled,
  onSubmit,
}: ActionFieldProps) {
  const [folderPath, setFolderPath] = useState(
    action?.type === 'open_folder' ? action.path : '',
  );
  const valid = folderPath.trim() !== '';

  function submit() {
    if (valid) onSubmit({ type: 'open_folder', path: folderPath.trim() });
  }

  return (
    <>
      <ActionTextInput
        label="Folder path"
        placeholder="C:\Users\me\Documents"
        value={folderPath}
        onChange={setFolderPath}
        disabled={disabled}
        onSubmit={submit}
      />
      <SetActionButton disabled={disabled || !valid} onClick={submit} />
    </>
  );
}
