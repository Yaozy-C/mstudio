import type { Project } from "../model";
import { savedValues, clipReceipt } from "../assistant/savedValues";
import { scriptReceipt } from "../assistant/scriptReceipt";
import { runsOf } from "../production/requestTask";
import { taskOutcome } from "../production/taskOutcome";
export function receipt(
  before: Project,
  after: Project,
  operations: Record<string, unknown>[],
  executionId: string,
) {
  return {
    applied: true,
    stage: "commit",
    outcome: "committed",
    executionId,
    savedValues: savedValues(after, operations),
    savedClips: clipReceipt(before, after),
    scripts: scriptReceipt(after, operations),
    updatedTasks: operations
      .filter((o) => o.op === "update_generation")
      .map((o) => {
        const task = after.production?.drafts?.[String(o.taskKey)];
        return task ? { ...taskOutcome(task), prompt: task.prompt } : null;
      }),
    generationTasks: runsOf(after)
      .filter(
        (t) =>
          t.key.startsWith(`run:${executionId}:`) ||
          t.lastRegenerationId?.startsWith(`${executionId}:`),
      )
      .map(taskOutcome),
    revision: after.revision,
    changed: operations.map((o) => ({
      op: o.op,
      id: o.id,
      fields: Object.keys(o).filter((key) => key !== "op" && key !== "id"),
    })),
    message:
      "Atomically saved. Receipt is authoritative; inspect only missing details or pixels. Generation tasks have their own execution status.",
  };
}
