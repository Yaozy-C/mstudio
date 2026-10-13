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

/** Recovery must reuse identical generation work, including standalone tasks. Different specifications remain valid variants. */
export function existingShotRun(
  runs: ProductionTask[],
  proposed: ProductionTask,
) {
  return runs.find(
    (task) =>
      task.targetNodeId === proposed.targetNodeId &&
      task.generationPurpose === proposed.generationPurpose &&
      task.kind === proposed.kind &&
      task.mode === proposed.mode &&
      task.modelId === proposed.modelId &&
      task.prompt === proposed.prompt &&
      canonical(mediaIdentity(task)) === canonical(mediaIdentity(proposed)) &&
      canonical(task.parameters ?? {}) === canonical(proposed.parameters ?? {}),
  );
}

// UI keys and layout metadata must not make identical media inputs look different.
function mediaIdentity(task: ProductionTask) {
  return task.inputs.map(
    ({ assetId, role, purpose, start, end, sourceRef }) => ({
      assetId,
      role,
      purpose,
      start,
      end,
      ...(role === "script" ? { sourceRef } : {}),
    }),
  );
}
