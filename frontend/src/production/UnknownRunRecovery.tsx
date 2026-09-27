import { t, useLanguage } from "../i18n";
import { useState } from "react";
import { Dialog } from "@radix-ui/themes";
import { runtime } from "../plugins/runtime";
import { ErrorNotice } from "../errors/ErrorNotice";
import type { ProductionTask } from "./types";
import type { ProductionController } from "./useProduction";
export function UnknownRunRecovery({
  task,
  canvas,
}: {
  task: ProductionTask;
  canvas: ProductionController;
}) {
  useLanguage();
  const [open, setOpen] = useState(false);
  const [confirmed, setConfirmed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>();
  return (
    <Dialog.Root
      open={open}
      onOpenChange={(value) => {
        if (!busy) {
          setOpen(value);
          setConfirmed(false);
          setError(undefined);
        }
      }}
    >
      <Dialog.Trigger>
        <button type="button">{t("核查后重新设置")}</button>
      </Dialog.Trigger>
      <Dialog.Content maxWidth="440px">
        <Dialog.Title>{t("先核查原任务")}</Dialog.Title>
        <Dialog.Description>
          {t(
            "请在服务商的任务记录中核查。若任务仍在运行，请继续等待；若已有结果，请先保留结果。重新生成可能产生新的费用。",
          )}
        </Dialog.Description>
        <p>
          {t("任务编号：")}
          <code>{task.requestId ?? task.jobId ?? t("尚未取得")}</code>
        </p>
        <label>
          <input
            type="checkbox"
            checked={confirmed}
            onChange={(e) => setConfirmed(e.target.checked)}
          />
          {t("我已确认原任务未执行或已终止，需要重新生成")}
        </label>
        <ErrorNotice error={error} fallback="JOB_SYNC_FAILED" />
        <div className="run-actions">
          <Dialog.Close>
            <button type="button" disabled={busy}>
              {t("返回")}
            </button>
          </Dialog.Close>
          <button
            type="button"
            disabled={!confirmed || busy}
            onClick={() => {
              setBusy(true);
              void (async () => {
                try {
                  if (task.jobId)
                    await runtime.execute("resolve_unknown_job", {
                      id: task.jobId,
                      confirmed,
                    });
                  const resolved = {
                    ...task,
                    status: "FAILED",
                    error: undefined,
                  };
                  canvas.update(resolved, task.key);
                  canvas.configure(resolved);
                  setOpen(false);
                } catch (e) {
                  setError(e);
                } finally {
                  setBusy(false);
                }
              })();
            }}
          >
            {busy ? t("正在处理…") : t("重新设置")}
          </button>
        </div>
      </Dialog.Content>
    </Dialog.Root>
  );
}
