import type { KeyAction } from '../api';

export interface ActionFieldProps {
  /** The currently committed action, used to seed this field's initial value. */
  action: KeyAction | undefined;
  disabled: boolean;
  onSubmit: (action: KeyAction) => void;
}
