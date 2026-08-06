import type { ActionType, KeyAction } from '../types';
import { DiscordJoinVoiceField } from './DiscordJoinVoiceField';
import { HotkeyField } from './HotkeyField';
import { MultiActionField } from './MultiActionField';
import { OpenFolderField } from './OpenFolderField';
import { OpenUrlField } from './OpenUrlField';
import { RunCommandField } from './RunCommandField';
import { TypeTextField } from './TypeTextField';

interface ActionTypeFieldProps {
  actionType: ActionType;
  action: KeyAction | undefined;
  disabled: boolean;
  onSubmit: (action: KeyAction) => void;
}

/** Renders the editor for whichever action type is currently selected. */
export function ActionTypeField({
  actionType,
  action,
  disabled,
  onSubmit,
}: ActionTypeFieldProps) {
  switch (actionType) {
    case 'run_command':
      return (
        <RunCommandField
          action={action}
          disabled={disabled}
          onSubmit={onSubmit}
        />
      );
    case 'open_url':
      return (
        <OpenUrlField action={action} disabled={disabled} onSubmit={onSubmit} />
      );
    case 'open_folder':
      return (
        <OpenFolderField
          action={action}
          disabled={disabled}
          onSubmit={onSubmit}
        />
      );
    case 'type_text':
      return (
        <TypeTextField
          action={action}
          disabled={disabled}
          onSubmit={onSubmit}
        />
      );
    case 'hotkey':
      return (
        <HotkeyField action={action} disabled={disabled} onSubmit={onSubmit} />
      );
    case 'discord_join_voice':
      return (
        <DiscordJoinVoiceField
          action={action}
          disabled={disabled}
          onSubmit={onSubmit}
        />
      );
    case 'multi':
      return (
        <MultiActionField
          action={action}
          disabled={disabled}
          onSubmit={onSubmit}
        />
      );
  }
}
