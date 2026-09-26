import { useState } from "react";
import { Select } from "@radix-ui/themes";
import "../styles/studio-select.css";
export type SelectOption = { value: string; label: string; disabled?: boolean };
export function StudioSelect({
  value,
  onValueChange,
  options,
  label,
  placeholder = "请选择",
  disabled = false,
}: {
  value: string;
  onValueChange: (value: string) => void;
  options: SelectOption[];
  label: string;
  placeholder?: string;
  disabled?: boolean;
}) {
  const [open, setOpen] = useState(false);
  const selected = options.some((option) => option.value === value);
  return (
    <Select.Root
      open={open}
      onOpenChange={setOpen}
      value={selected ? `option:${value}` : ""}
      onValueChange={(next) => onValueChange(next.slice(7))}
      disabled={disabled}
    >
      <Select.Trigger
        className="studio-select-trigger"
        aria-label={label}
        placeholder={placeholder}
      />
      <Select.Content
        className="studio-select-menu"
        position="popper"
        sideOffset={6}
        align="start"
      >
        {options.map((option) => (
          <Select.Item
            key={option.value}
            value={`option:${option.value}`}
            disabled={option.disabled}
            onKeyDown={(event) => {
              if (event.key !== "Enter" || option.disabled) return;
              // A switched form may autofocus an input during this key event.
              // Cancel the browser's implicit submit before changing selection.
              event.preventDefault();
              event.stopPropagation();
              onValueChange(option.value);
              setOpen(false);
            }}
          >
            {option.label}
          </Select.Item>
        ))}
        {!options.length && (
          <Select.Item value="empty" disabled>
            暂无可选项
          </Select.Item>
        )}
      </Select.Content>
    </Select.Root>
  );
}
