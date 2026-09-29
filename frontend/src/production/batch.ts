import { type Project } from "../model";
import { saveTask } from "./document";
import { retryTask } from "./recovery";
import { runsOf } from "./requestTask";
import type { ProductionTask } from "./types";

// Derive progress from durable tasks, never from the model's conversation memory.
export function batchSummary(tasks: ProductionTask[]) {
  const counts: Record<string, number> = {};
  let attention = 0;
  for (const task of tasks) {
    const status = task.status ?? "AWAITING_CONFIRMATION";
    counts[status] = (counts[status] ?? 0) + 1;
    if (task.trackingPaused || status === "UNKNOWN" || status === "FAILED")
      attention++;
  }
  return { total: tasks.length, counts, attention };
}
export function changeBatch(
  p: Project,
  turnId: string,
  action: "resume" | "stop" | "retry",
) {
  let next = p;
  const tasks = runsOf(p, turnId);
  for (const task of tasks) {
    if (action === "stop" && task.status === "READY")
      next = saveTask(next, { ...task, status: "CANCELLED" });
    if (
      action === "resume" &&
      task.status === "AWAITING_CONFIRMATION" &&
      task.modelId &&
      !task.jobId &&
      !task.submissionId
    )
      next = saveTask(next, { ...task, status: "READY", error: undefined });
    // Reset the same task. Unknown outcomes and partial results are not replayed.
    if (
      action === "retry" &&
      task.status === "FAILED" &&
      task.modelId &&
      !task.resultAssetId &&
      !task.resultAssetIds?.length
    ) {
      const retry = retryTask(task);
      next = saveTask(next, {
        ...retry,
        status: "READY",
      });
    }
  }
  return next;
}
