import { resultPlacement } from "./resultPlacement";
import { saveTask } from "./document";
import { taskKey } from "./tasks";
import type { Project } from "../model";
import type { MediaModel } from "../models/mediaRegistry";
import type { ProductionTask } from "./types";
import { canvasSnapshot, inputFor } from "./request";

export function directTask(
  project: Project,
  draft: ProductionTask,
  model: MediaModel,
  id: string,
): ProductionTask {
  if (draft.inputs.some((r) => r.role === "script"))
    throw new Error("文字资料请在 Agent 模式使用，或移除后生成");
  const task: ProductionTask = {
    key: `run:direct:${id}`,
    turnId: `direct:${id}`,
    createdAt: Date.now(),
    kind: draft.kind,
    mode: draft.mode,
    modelId: model.id,
    ownerId: undefined,
    position: draft.position ?? resultPlacement(project, draft),
    prompt: draft.prompt.trim(),
    instruction: draft.prompt.trim(),
    parameters: structuredClone(draft.parameters),
    inputs: structuredClone(draft.inputs),
    status: "READY",
  };
  if (task.prompt.length > 12000) throw new Error("生成描述最多 12000 字");
  canvasSnapshot(project, task, { x: 0, y: 0 });
  inputFor(project, task, model);
  return task;
}

// Clear only the submitted draft; the durable run owns an independent input snapshot.
export function acceptDirectTask(
  project: Project,
  draft: ProductionTask,
  run: ProductionTask,
): Project {
  const empty: ProductionTask = {
    key: draft.key,
    kind: draft.kind,
    modelId: run.modelId,
    mode: draft.kind === "video" ? "single" : "multi",
    inputs: [],
    prompt: "",
    parameters: structuredClone(draft.parameters),
  };
  return saveTask(saveTask(saveTask(project, run), empty), {
    ...empty,
    key: taskKey([]),
  });
}
