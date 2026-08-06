interface ActionPayloads {
  run_command: { command: string };
  open_url: { url: string };
  open_folder: { path: string };
  type_text: { text: string };
  hotkey: { keys: string[] };
  discord_join_voice: { channel_id: string };
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
  discord_join_voice: 'Join Discord voice channel',
  multi: 'Multiple actions',
} satisfies Record<ActionType, string>;

export const ACTION_TYPES = Object.entries(ACTION_LABELS).map(
  ([value, label]) => ({
    value: value as ActionType,
    label,
  }),
);

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
    case 'discord_join_voice':
      return action.channel_id;
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
