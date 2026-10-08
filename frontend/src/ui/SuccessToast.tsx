import { useEffect, useEffectEvent } from "react";
import { Portal } from "@radix-ui/themes";
import { CheckCircle, X } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import "../styles/success-toast.css";

export function SuccessToast({
  message,
  onDismiss,
}: {
  message: string;
  onDismiss: () => void;
}) {
  useLanguage();
  const dismiss = useEffectEvent(onDismiss);
  useEffect(() => {
    if (!message) return;
    const timer = window.setTimeout(() => dismiss(), 5000);
    return () => window.clearTimeout(timer);
  }, [message]);
  if (!message) return null;
  return (
    <Portal>
      <div className="success-toast" role="status" aria-atomic="true">
        <CheckCircle size={18} aria-hidden="true" />
        <span>{message}</span>
        <button type="button" aria-label={t("关闭提示")} onClick={onDismiss}>
          <X size={16} />
        </button>
      </div>
    </Portal>
  );
}
