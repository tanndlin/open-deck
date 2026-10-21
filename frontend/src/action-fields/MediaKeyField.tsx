import { useState } from 'react';
import { MEDIA_KEYS } from '../types';
import { SetActionButton } from './SetActionButton';
import type { ActionFieldProps } from './types';

export function MediaKeyField({
  action,
  disabled,
  onSubmit,
}: ActionFieldProps) {
  const [key, setKey] = useState(
    action?.type === 'media_key' ? action.key : 'playpause',
  );

  return (
    <>
      <label className="flex flex-col gap-1.5 text-[13px] text-text">
        Key
        <select
          className="rounded-sm border border-border bg-bg px-2 py-1.5 text-sm text-text-h"
          value={key}
          disabled={disabled}
          onChange={(e) => setKey(e.target.value)}
        >
          {MEDIA_KEYS.map((k) => (
            <option key={k.value} value={k.value}>
              {k.label}
            </option>
          ))}
        </select>
      </label>
      <SetActionButton
        disabled={disabled}
        onClick={() => onSubmit({ type: 'media_key', key })}
      />
    </>
  );
}
