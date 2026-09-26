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
  return (
    <button
      type="button"
      className="generation-task-entry"
      onClick={() => canvas.showTask(task)}
    >
      <span>生成{task.kind === "image" ? "图片" : "视频"}</span>
      <span>{runStatuses[task.status ?? ""] ?? "待开始"}</span>
      <span>查看任务 ↗</span>
    </button>
  );
}
