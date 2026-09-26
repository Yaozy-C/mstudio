import { useState, type ReactNode } from "react";
import { normalizeError, type ErrorCode } from "./catalog";
import "./errors.css";
export function ErrorNotice({
  error,
  fallback,
  children,
  taskId,
}: {
  error: unknown;
  fallback?: ErrorCode;
  children?: ReactNode;
  taskId?: string;
}) {
  const [copyState, setCopyState] = useState("");
  if (!error) return null;
  const item = normalizeError(error, fallback);
  const details = [
    item.code,
    item.httpStatus && `HTTP ${item.httpStatus}`,
    taskId && `任务：${taskId}`,
    item.details,
  ]
    .filter(Boolean)
    .join("\n");
  return (
    <div className="error error-notice" role="alert">
      <strong>{item.message}</strong>
      <span>{item.recovery}</span>
      {children && <div className="error-notice-actions">{children}</div>}
      <details>
        <summary>错误详情 · {item.code}</summary>
        <pre>{details}</pre>
        <button
          type="button"
          onClick={() => {
            void navigator.clipboard.writeText(details).then(
              () => setCopyState("已复制"),
              () => setCopyState("复制失败，请选择上方详情复制"),
            );
          }}
        >
          复制错误详情
        </button>
        <small role="status">{copyState}</small>
      </details>
    </div>
  );
}
