import { regenerationDraft } from "./taskEditing";
import type { ProductionTask } from "./types";
export const pendingStatuses = [
  "READY",
  "UPLOADING",
  "SUBMITTING",
  "IN_QUEUE",
  "IN_PROGRESS",
  "RECEIVING",
  "CANCEL_REQUESTED",
];
export function retryTask(task: ProductionTask): ProductionTask {
  if (!["FAILED", "CANCELLED", "UNKNOWN"].includes(task.status ?? ""))
    throw new Error("任务尚未结束，请稍后重试");
  return regenerationDraft(task);
}
