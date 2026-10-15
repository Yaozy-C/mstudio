import type { ComponentProps } from "react";
import type { Icon } from "@phosphor-icons/react";
import { StatusIcon } from "./AsyncState";

/** Icon-only actions always carry the same visible tooltip and accessible name. */
export function IconButton({
  icon: Icon,
  label,
  busy = false,
  disabled,
  className = "",
  type = "button",
  ...props
}: Omit<ComponentProps<"button">, "children"> & {
  icon: Icon;
  label: string;
  busy?: boolean;
}) {
  return (
    <button
      type={type}
      title={label}
      aria-label={label}
      {...props}
      disabled={disabled || busy}
      aria-busy={busy}
      className={`icon-button ${className}`.trim()}
    >
      {busy ? <StatusIcon kind="loading" /> : <Icon aria-hidden="true" />}
    </button>
  );
}
