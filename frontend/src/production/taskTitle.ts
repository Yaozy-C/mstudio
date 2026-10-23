import type { ProductionTask } from "./types";

export function checkedTaskTitle(value: unknown): string {
  if (typeof value !== "string" || !value.trim() || value.length > 100)
    throw new Error("请提供具体任务名称（最多 100 字）");
  return value.trim();
}

export function taskTitle(task: Pick<ProductionTask, "title">): string {
  return checkedTaskTitle(task.title);
}
