interface SetActionButtonProps {
  disabled: boolean;
  onClick: () => void;
}

/** Shared "Set action" button used by the per-action-type fields below. */
export function SetActionButton({ disabled, onClick }: SetActionButtonProps) {
  return (
    <div className="flex gap-2">
      <button
        type="button"
        className="cursor-pointer rounded-sm border border-border bg-code-bg px-3 py-1.5 text-sm text-text-h disabled:cursor-not-allowed disabled:opacity-50"
        disabled={disabled}
        onClick={onClick}
      >
        Set action
      </button>
    </div>
  );
}
