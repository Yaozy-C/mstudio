import type { ProductionTask } from "./types";

// Fold the former editable field into the single prompt without losing saved edits.
export function normalizeTaskPrompt(task: ProductionTask): ProductionTask {
  if (!("nextPrompt" in task)) return task;
  const { nextPrompt, ...rest } = task as ProductionTask & {
    nextPrompt?: unknown;
  };
  return {
    ...rest,
    prompt:
      typeof nextPrompt === "string" && nextPrompt.trim()
        ? nextPrompt
        : task.prompt,
  };
}
