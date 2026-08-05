interface ActionPayloads {
  run_command: { command: string };
  open_url: { url: string };
  open_folder: { path: string };
  type_text: { text: string };
  hotkey: { keys: string[] };
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
} satisfies Record<ActionType, string>;

export const ACTION_TYPES = Object.entries(ACTION_LABELS).map(
  ([value, label]) => ({
    value: value as ActionType,
    label,
  }),
);

export interface KeyConfig {
  icon?: string;
  title?: string;
  action?: KeyAction;
  is_folder: boolean;
}

export type KeyMap = Record<string, KeyConfig>;

/** A sequence of key indices followed from the home page. Empty is home. */
export type PagePath = number[];
