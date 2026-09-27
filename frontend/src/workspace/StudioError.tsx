import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { X } from "@phosphor-icons/react";
export function StudioError({
  message,
  close,
}: {
  message: string;
  close: () => void;
}) {
  useLanguage();
  return message ? (
    <div className="floating-error">
      <ErrorNotice error={message} />
      <button aria-label={t("关闭错误提示")} onClick={close}>
        <X />
      </button>
    </div>
  ) : null;
}
