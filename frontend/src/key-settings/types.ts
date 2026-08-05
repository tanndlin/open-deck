import type { KeyAction } from '../types';

export interface KeySettingsActions {
  onSetIcon: (id: number, path: string) => Promise<void>;
  onClearIcon: (id: number) => Promise<void>;
  onSetTitle: (id: number, title: string) => Promise<void>;
  onClearTitle: (id: number) => Promise<void>;
  onSetAction: (id: number, action: KeyAction) => Promise<void>;
  onClearAction: (id: number) => Promise<void>;
  onMakeFolder: (id: number) => Promise<void>;
  onRemoveFolder: (id: number) => Promise<void>;
  onOpenFolder: (id: number) => void;
}
