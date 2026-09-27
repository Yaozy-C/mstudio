import { t, useLanguage } from "../i18n";
import type { ProductionTask } from "./types";
import type { ProductionController } from "./useProduction";
import { runStatuses } from "./useRunActions";
import "./task-panel.css";
export function TaskEntry({
  task,
  canvas,
}: {
  task: ProductionTask;
  canvas: ProductionController;
}) {
  useLanguage();
  return (
    <button
      type="button"
      className="generation-task-entry"
      onClick={() => canvas.showTask(task)}
    >
      <span>
        {t("生成")}
        {task.kind === "image" ? t("图片") : t("视频")}
      </span>
      <span>{t(runStatuses[task.status ?? ""] ?? "") || t("待开始")}</span>
      <span>{t("查看任务 ↗")}</span>
    </button>
  );
}
