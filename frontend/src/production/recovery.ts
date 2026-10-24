import { regenerationDraft } from "./taskEditing";
import type { ProductionTask } from "./types";
export const pendingStatuses = [
  "READY",
  "UPLOADING",
  "SUBMITTING",
  "IN_QUEUE",
  "IN_PROGRESS",
  "RECEIVING",
  "UNKNOWN",
  "CANCEL_REQUESTED",
];
export function retryTask(task: ProductionTask): ProductionTask {
  if (!["FAILED", "CANCELLED"].includes(task.status ?? ""))
    throw new Error("原任务状态尚未确认，请先核查结果");
  return regenerationDraft(task);
}
