import type { Project } from "../model";
import type { ProductionTask } from "./types";
import { saveTask } from "./document";
import { pendingStatuses } from "./recovery";
import { taskTitle } from "./taskTitle";

export function archiveTask(p: Project, key: string, archived: boolean) {
  const task = p.production?.drafts?.[key];
  if (!task) throw new Error("生成任务不存在");
  if (typeof archived !== "boolean") throw new Error("归档状态无效");
  if (archived && pendingStatuses.includes(task.status ?? ""))
    throw new Error("请先停止生成或等待任务结束，再归档");
  return saveTask(p, { ...task, hiddenFromList: archived });
}
export function archivedSummary(p: Project, task: ProductionTask) {
  const ids = [
    ...new Set(
      [...(task.resultAssetIds ?? []), task.resultAssetId].filter(
        (id): id is string => !!id,
      ),
    ),
  ];
  return {
    id: task.key,
    title: taskTitle(task),
    kind: task.kind,
    summary: task.instruction?.slice(0, 120),
    targetNodeId: task.targetNodeId,
    status: task.status,
    archived: true,
    hiddenFromList: true,
    resultAssetIds: ids.filter((id) =>
      p.assets.some((a) => a.id === id && !a.discarded),
    ),
    discardedResultCount: ids.filter((id) =>
      p.assets.some((a) => a.id === id && a.discarded),
    ).length,
    continuation: {
      state: "finished",
      action: "read_details_on_request",
      trigger: "new_user_request",
    },
  };
}
