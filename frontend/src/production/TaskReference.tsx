import { t, useLanguage } from "../i18n";
import type { ProductionController } from "./useProduction";
export function TaskReference({ canvas }: { canvas: ProductionController }) {
  useLanguage();
  return (
    <>
      {canvas.referencedTask && (
        <div className="composer-task-reference">
          <button
            type="button"
            onClick={() => canvas.showTask(canvas.referencedTask!)}
          >
            {t("已引用")}
            {canvas.referencedTask.kind === "image" ? t("图片") : t("视频")}
            {t("任务 ·")} {canvas.referencedTask.prompt.slice(0, 32)}
          </button>
          <button
            type="button"
            aria-label={t("移除任务引用")}
            onClick={() => canvas.clearTaskReference()}
          >
            ×
          </button>
        </div>
      )}
    </>
  );
}
