import { ArrowUpRight, Image, VideoCamera } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import type { ProductionTask } from "./types";
import type { ProductionController } from "./useProduction";
import { RunStatus } from "./RunStatus";
import { taskTitle } from "./taskTitle";
import "./task-panel.css";
export function TaskEntry({
  task,
  canvas,
}: {
  task: ProductionTask;
  canvas: ProductionController;
}) {
  useLanguage();
  const Icon = task.kind === "image" ? Image : VideoCamera;
  return (
    <button
      type="button"
      className="generation-task-entry"
      onClick={() => canvas.showTask(task)}
    >
      <Icon size={20} aria-hidden="true" />
      <span className="task-entry-copy">
        <strong>{taskTitle(task)}</strong>
        <span className="task-entry-prompt">{task.prompt}</span>
        <span className="task-entry-status" data-status={task.status}>
          <RunStatus task={task} />
        </span>
      </span>
      <span className="task-entry-open">
        {t("查看任务")}
        <ArrowUpRight size={14} aria-hidden="true" />
      </span>
    </button>
  );
}
