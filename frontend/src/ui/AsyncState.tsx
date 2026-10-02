import type { ComponentProps, ReactNode } from "react";
import {
  CircleNotch,
  CheckCircle,
  Info,
  WarningCircle,
  PauseCircle,
} from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import "../styles/async-state.css";
export type StatusKind = "loading" | "success" | "error" | "info" | "paused";
const icons = {
  loading: CircleNotch,
  success: CheckCircle,
  error: WarningCircle,
  info: Info,
  paused: PauseCircle,
};
export function StatusIcon({
  kind,
  size = 18,
}: {
  kind: StatusKind;
  size?: number;
}) {
  const Icon = icons[kind];
  return (
    <Icon
      size={size}
      className={kind === "loading" ? "status-spinner" : undefined}
      aria-hidden="true"
    />
  );
}
export function StatusMessage({
  kind = "loading",
  children,
  className = "",
}: {
  kind?: StatusKind;
  children: ReactNode;
  className?: string;
}) {
  return (
    <div
      className={`status-message ${className}`}
      data-kind={kind}
      role={kind === "error" ? "alert" : "status"}
      aria-atomic="true"
    >
      <StatusIcon kind={kind} />
      <span>{children}</span>
    </div>
  );
}
/** Only use on initial loads. Refreshes keep existing content visible. */
export function LoadingState({
  label,
  rows = 3,
}: {
  label: string;
  rows?: number;
}) {
  return (
    <div className="loading-state" aria-busy="true">
      <StatusMessage>{label}</StatusMessage>
      <div className="loading-skeleton" aria-hidden="true">
        {Array.from({ length: rows }, (_, i) => (
          <div key={i} className="loading-skeleton-row">
            <span />
            <span />
          </div>
        ))}
      </div>
    </div>
  );
}
/** Reserve both labels' width, so submitting never shifts nearby actions. */
export function AsyncButton({
  busy = false,
  busyLabel,
  children,
  disabled,
  className = "",
  type = "button",
  ...props
}: ComponentProps<"button"> & { busy?: boolean; busyLabel?: string }) {
  useLanguage();
  return (
    <button
      {...props}
      type={type}
      disabled={disabled || busy}
      aria-busy={busy}
      className={`async-button ${className}`}
    >
      <span
        className="async-button-label"
        aria-hidden={busy}
        data-hidden={busy}
      >
        {children}
      </span>
      <span
        className="async-button-pending"
        aria-hidden={!busy}
        data-hidden={!busy}
      >
        <StatusIcon kind="loading" size={16} />
        {busyLabel || t("处理中…")}
      </span>
    </button>
  );
}
