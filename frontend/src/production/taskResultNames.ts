import type { Project } from "../model";
import type { ProductionSource, ProductionTask } from "./types";
import { taskTitle } from "./taskTitle";

export function resultTask(
  p: Project,
  source: ProductionSource,
): ProductionTask {
  if (!source?.canvasGeneration)
    throw new Error("生成任务缺少制作上下文，未写入项目");
  const snapshot = source.canvasGeneration.task;
  return p.production?.drafts?.[snapshot.key] ?? snapshot;
}

export function resultTaskName(p: Project, source: ProductionSource): string {
  return taskTitle(resultTask(p, source));
}
