import { StatusIcon } from "./AsyncState";
import type { ComponentProps } from "react";
import type { Icon } from "@phosphor-icons/react";
import "../styles/action-button.css";

/** Inline actions share one visual contract across editors and panels. */
export function ActionButton({
  icon: Icon,
  busy = false,
  busyLabel,
  disabled,
  className = "",
  children,
  type = "button",
  ...props
}: ComponentProps<"button"> & {
  icon: Icon;
  busy?: boolean;
  busyLabel?: string;
}) {
  return (
    <button
      {...props}
      type={type}
      disabled={disabled || busy}
      aria-busy={busy}
      className={`action-button ${className}`.trim()}
    >
      {busy ? (
        <StatusIcon kind="loading" size={20} />
      ) : (
        <Icon size={20} weight="regular" aria-hidden="true" />
      )}
      <span>{busy && busyLabel ? busyLabel : children}</span>
    </button>
  );
}
