import type { Project } from "../model";
import { saveTask } from "./document";
import type { ProductionTask } from "./types";

export const canEditOriginal = (task: ProductionTask) =>
  !task.jobId &&
  !task.submissionId &&
  (!task.status || task.status === "AWAITING_CONFIRMATION");
export const canRegenerate = (task: ProductionTask) =>
  ["COMPLETED", "FAILED", "CANCELLED", "IN_QUEUE", "IN_PROGRESS"].includes(
    task.status ?? "",
  );
export function editTaskPrompt(p: Project, key: string, text: string): Project {
  const task = p.production?.drafts?.[key];
  if (!task) throw new Error("任务不存在，请重新查询");
  if (!text.trim() || text.length > 12000)
    throw new Error("请提供完整生成描述（最多 12000 字）");
  return saveTask(
    p,
    canEditOriginal(task)
      ? { ...task, prompt: text.trim(), error: undefined }
      : { ...task, nextPrompt: text.trim() },
  );
}
export function regenerationDraft(
  task: ProductionTask,
  id: string,
): ProductionTask {
  if (!canRegenerate(task)) throw new Error("请先核查原任务状态，再重新生成");
  return {
    ...structuredClone(task),
    key: `retry:${id}`,
    sourceTaskKey: task.key,
    prompt: task.nextPrompt ?? task.prompt,
    nextPrompt: undefined,
    hiddenFromList: false,
    status: "AWAITING_CONFIRMATION",
    jobId: undefined,
    submissionId: undefined,
    requestId: undefined,
    error: undefined,
    trackingPaused: undefined,
    progress: undefined,
    resultAssetId: undefined,
    resultAssetIds: undefined,
    createdAt: Date.now(),
  };
}
