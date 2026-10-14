import { StatusIcon } from "../ui/AsyncState";
import { t, useLanguage } from "../i18n";
import { useState, type ReactNode } from "react";
import { CaretDown, CopySimple } from "@phosphor-icons/react";
import { normalizeError, errorCatalog, type ErrorCode } from "./catalog";
import "./errors.css";
export function ErrorNotice({
  error,
  fallback,
  children,
  taskId,
  variant = "default",
}: {
  error: unknown;
  fallback?: ErrorCode;
  children?: ReactNode;
  taskId?: string;
  variant?: "default" | "conversation";
}) {
  const language = useLanguage();
  const [copyState, setCopyState] = useState("");
  const noticeClass = variant === "conversation" ? " conversation-notice" : "";
  if (!error) return null;
  const item = normalizeError(error, fallback);
  const localize = (value: string, fallback: string) => {
    const translated = t(value);
    return language === "en" && /[\u3400-\u9fff]/.test(translated)
      ? t(fallback)
      : translated;
  };
  const message = localize(item.message, errorCatalog[item.code][0]);
  const recovery = localize(item.recovery, errorCatalog[item.code][1]);
  if (item.code === "CHAT_STOPPED")
    return (
      <div className={`operation-stopped${noticeClass}`} role="status">
        <strong className="error-notice-heading">
          <StatusIcon kind="paused" />
          {message}
        </strong>
        <span>{recovery}</span>
        {children && <div className="error-notice-actions">{children}</div>}
      </div>
    );
  const details = [
    item.code,
    item.httpStatus && `HTTP ${item.httpStatus}`,
    taskId && t("任务：{v0}", { v0: taskId }),
    message !== t(item.message) ? item.message : undefined,
    item.details,
  ]
    .filter(Boolean)
    .join("\n");
  return (
    <div className={`error error-notice${noticeClass}`} role="alert">
      <strong className="error-notice-heading">
        <StatusIcon kind="error" />
        {message}
      </strong>
      <span className="error-notice-recovery">{recovery}</span>
      <div className="error-notice-footer">
        {children && <div className="error-notice-actions">{children}</div>}
        <details className="error-notice-details">
          <summary>
            {t("错误详情")}
            <CaretDown size={12} aria-hidden="true" />
          </summary>
          <div className="error-notice-diagnostics">
            <pre>{details}</pre>
            <div className="error-notice-copy">
              <button
                type="button"
                onClick={() => {
                  void navigator.clipboard.writeText(details).then(
                    () => setCopyState(t("已复制")),
                    () => setCopyState(t("复制失败，请选择上方详情复制")),
                  );
                }}
              >
                <CopySimple size={14} aria-hidden="true" />
                {t("复制错误详情")}
              </button>
              <small role="status">{copyState}</small>
            </div>
          </div>
        </details>
      </div>
    </div>
  );
}
