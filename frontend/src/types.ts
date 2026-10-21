interface ActionPayloads {
  run_command: { command: string };
  open_url: { url: string };
  open_folder: { path: string };
  type_text: { text: string };
  hotkey: { keys: string[] };
  media_key: { key: string };
  http_request: {
    method: string;
    url: string;
    headers: { name: string; value: string }[];
    body: string;
  };
  discord_join_voice: { channel_id: string };
  discord_toggle_mute: Record<never, never>;
  discord_toggle_deafen: Record<never, never>;
  multi: { actions: KeyAction[] };
}

export type KeyAction = {
  [K in keyof ActionPayloads]: { type: K } & ActionPayloads[K];
}[keyof ActionPayloads];

export type ActionType = keyof ActionPayloads;

// still need RunCommandAction etc. elsewhere? derive them instead of hand-writing:
export type RunCommandAction = Extract<KeyAction, { type: 'run_command' }>;

const ACTION_LABELS = {
  run_command: 'Run command',
  open_url: 'Open webpage',
  open_folder: 'Open folder',
  type_text: 'Type text',
  hotkey: 'Hotkey',
  media_key: 'Media key',
  http_request: 'HTTP request',
  discord_join_voice: 'Join Discord voice channel',
  discord_toggle_mute: 'Toggle Discord mute',
  discord_toggle_deafen: 'Toggle Discord deafen',
  multi: 'Multiple actions',
} satisfies Record<ActionType, string>;

export const ACTION_TYPES = Object.entries(ACTION_LABELS).map(
  ([value, label]) => ({
    value: value as ActionType,
    label,
  }),
);

export const MEDIA_KEYS = [
  { value: 'playpause', label: 'Play / pause' },
  { value: 'next', label: 'Next track' },
  { value: 'prev', label: 'Previous track' },
  { value: 'stop', label: 'Stop' },
  { value: 'volumeup', label: 'Volume up' },
  { value: 'volumedown', label: 'Volume down' },
  { value: 'mute', label: 'Mute system audio' },
];

/** Short one-line preview of an action's payload, for compact list rows. */
export function actionSummary(action: KeyAction | undefined): string {
  if (!action) return 'No action set';
  switch (action.type) {
    case 'run_command':
      return action.command;
    case 'open_url':
      return action.url;
    case 'open_folder':
      return action.path;
    case 'type_text':
      return action.text;
    case 'hotkey':
      return action.keys.join('+');
    case 'media_key':
      return (
        MEDIA_KEYS.find((k) => k.value === action.key)?.label ?? action.key
      );
    case 'http_request':
      return `${action.method} ${action.url}`;
    case 'discord_join_voice':
      return action.channel_id;
    case 'discord_toggle_mute':
    case 'discord_toggle_deafen':
      return '';
    case 'multi':
      return `${action.actions.length} action${action.actions.length === 1 ? '' : 's'}`;
  }
}

export interface KeyConfig {
  icon?: string;
  title?: string;
  action?: KeyAction;
  is_folder: boolean;
}

export type KeyMap = Record<string, KeyConfig>;

/** A sequence of key indices followed from the home page. Empty is home. */
export type PagePath = number[];
