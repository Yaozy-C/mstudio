import type { ProductionTask } from "./types";

export function normalizeTaskTitle(value: unknown, unnamed = "未命名"): string {
  return typeof value === "string" && value.trim() ? value.trim() : unnamed;
}

export function taskTitle(
  task: Pick<ProductionTask, "title">,
  unnamed = "未命名",
): string {
  return normalizeTaskTitle(task.title, unnamed);
}
