interface ActionTextInputProps {
  label: string;
  placeholder: string;
  value: string;
  disabled: boolean;
  onChange: (value: string) => void;
  onSubmit: () => void;
}

/** Shared text input used by the per-action-type fields below. */
export function ActionTextInput({
  label,
  placeholder,
  value,
  disabled,
  onChange,
  onSubmit,
}: ActionTextInputProps) {
  return (
    <label className="flex flex-col gap-1.5 text-[13px] text-text">
      {label}
      <input
        type="text"
        className="rounded-sm border border-border bg-bg px-2 py-1.5 font-mono text-sm text-text-h"
        value={value}
        placeholder={placeholder}
        disabled={disabled}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter') {
            e.preventDefault();
            onSubmit();
          }
        }}
      />
    </label>
  );
}
