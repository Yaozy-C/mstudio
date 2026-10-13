import { useEffect, useState } from "react";
import { Check, Copy } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";

export function MessageCopyButton({ text }: { text: string }) {
  useLanguage();
  const [status, setStatus] = useState<"idle" | "copied" | "failed">("idle");
  useEffect(() => setStatus("idle"), [text]);
  useEffect(() => {
    if (status !== "copied") return;
    const timer = setTimeout(() => setStatus("idle"), 2000);
    return () => clearTimeout(timer);
  }, [status]);
  const Icon = status === "copied" ? Check : Copy;
  return (
    <div className="agent-message-actions">
      <button
        type="button"
        className="agent-message-copy"
        aria-label={t("复制消息")}
        title={t("复制消息")}
        onClick={() => {
          // Keep the clipboard write in the click's user gesture for WebKit.
          try {
            void navigator.clipboard.writeText(text).then(
              () => setStatus("copied"),
              () => setStatus("failed"),
            );
          } catch {
            setStatus("failed");
          }
        }}
      >
        <Icon size={14} aria-hidden="true" />
        {status === "copied" ? t("已复制") : t("复制")}
      </button>
      <span role="status">
        {status === "failed" && t("复制失败，请选中文字后按 ⌘C / Ctrl+C。")}
      </span>
    </div>
  );
}
