import { useState } from 'react';
import { ActionTextInput } from './ActionTextInput';
import { SetActionButton } from './SetActionButton';
import type { ActionFieldProps } from './types';

export function DiscordJoinVoiceField({
  action,
  disabled,
  onSubmit,
}: ActionFieldProps) {
  const [channelId, setChannelId] = useState(
    action?.type === 'discord_join_voice' ? action.channel_id : '',
  );
  const valid = channelId.trim() !== '';

  function submit() {
    if (valid)
      onSubmit({ type: 'discord_join_voice', channel_id: channelId.trim() });
  }

  return (
    <>
      <ActionTextInput
        label="Voice channel ID"
        placeholder="123456789012345678"
        value={channelId}
        onChange={setChannelId}
        disabled={disabled}
        onSubmit={submit}
      />
      <SetActionButton disabled={disabled || !valid} onClick={submit} />
    </>
  );
}
