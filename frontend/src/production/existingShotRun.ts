import type { ProductionTask } from "./types";

function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(",")}]`;
  if (value && typeof value === "object")
    return `{${Object.entries(value)
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([k, v]) => `${JSON.stringify(k)}:${canonical(v)}`)
      .join(",")}}`;
  return JSON.stringify(value) ?? "null";
}

/** Recovery must reuse identical shot work. Different specifications remain valid variants. */
export function existingShotRun(
  runs: ProductionTask[],
  proposed: ProductionTask,
) {
  if (!proposed.targetNodeId) return undefined;
  return runs.find(
    (task) =>
      task.targetNodeId === proposed.targetNodeId &&
      task.kind === proposed.kind &&
      task.mode === proposed.mode &&
      task.modelId === proposed.modelId &&
      task.prompt === proposed.prompt &&
      canonical(task.inputs) === canonical(proposed.inputs) &&
      canonical(task.parameters ?? {}) === canonical(proposed.parameters ?? {}),
  );
}
