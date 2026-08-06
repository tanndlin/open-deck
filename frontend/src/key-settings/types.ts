export interface KeySettingsActions {
  onRemoveFolder: (id: number) => Promise<void>;
  onOpenFolder: (id: number) => void;
}
