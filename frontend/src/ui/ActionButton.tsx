import type { ComponentProps } from "react";
import type { Icon } from "@phosphor-icons/react";
import "../styles/action-button.css";

/** Inline actions share one visual contract across editors and panels. */
export function ActionButton({
  icon: Icon,
  className = "",
  children,
  type = "button",
  ...props
}: ComponentProps<"button"> & { icon: Icon }) {
  return (
    <button
      {...props}
      type={type}
      className={`action-button ${className}`.trim()}
    >
      <Icon size={20} weight="regular" aria-hidden="true" />
      <span>{children}</span>
    </button>
  );
}
