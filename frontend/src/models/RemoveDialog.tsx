import { AlertDialog } from "@radix-ui/themes";
import type { ReactNode } from "react";
import { t } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
export function RemoveDialog({
  open,
  onOpenChange,
  trigger,
  title,
  children,
  busy,
  error,
  confirm,
  blocked = false,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  trigger: ReactNode;
  title: string;
  children: ReactNode;
  busy: boolean;
  error: string;
  confirm: () => void;
  blocked?: boolean;
}) {
  return (
    <AlertDialog.Root
      open={open}
      onOpenChange={(next) => {
        if (!busy) onOpenChange(next);
      }}
    >
      <AlertDialog.Trigger>{trigger}</AlertDialog.Trigger>
      <AlertDialog.Content className="modal small" aria-busy={busy}>
        <AlertDialog.Title>{title}</AlertDialog.Title>
        <AlertDialog.Description>{children}</AlertDialog.Description>
        {error && <ErrorNotice error={error} fallback="OPERATION_FAILED" />}
        <footer>
          <AlertDialog.Cancel>
            <button type="button" disabled={busy}>
              {blocked ? t("关闭") : t("取消")}
            </button>
          </AlertDialog.Cancel>
          {!blocked && (
            <button
              type="button"
              className="primary"
              disabled={busy}
              onClick={confirm}
            >
              {busy ? t("正在移除…") : t("确认移除")}
            </button>
          )}
        </footer>
      </AlertDialog.Content>
    </AlertDialog.Root>
  );
}
