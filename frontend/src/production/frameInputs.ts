import type { Project } from "../model";
import { mediaAdapter } from "../models/adapters";
import type { MediaModel } from "../models/mediaRegistry";
import type { ProductionInput, ProductionTask } from "./types";

export type FrameRole = "first-frame" | "last-frame";
export function frameRoles(model?: MediaModel): FrameRole[] {
  if (!model || model.kind !== "video") return [];
  const fields = mediaAdapter(model).fields;
  return (["first-frame", "last-frame"] as const).filter((role) =>
    fields.some((f) => f.role === role),
  );
}
export function setFrameInput(
  task: ProductionTask,
  project: Project,
  role: FrameRole,
  input?: ProductionInput,
): Partial<ProductionTask> {
  if (input) {
    const asset = project.assets.find((a) => a.id === input.assetId);
    if (!asset || asset.kind !== "image" || asset.missing)
      throw new Error("首尾帧只能选择可用的图片");
    if (
      task.inputs.some(
        (r) =>
          r.assetId === input.assetId &&
          r.role !== role &&
          (r.role === "first-frame" || r.role === "last-frame"),
      )
    )
      throw new Error("首帧和尾帧请选择不同的图片");
  }
  const inputs = task.inputs.filter(
    (r) => r.role !== role && (!input || r.assetId !== input.assetId),
  );
  if (input)
    inputs.push({
      ...input,
      role,
      purpose: role === "first-frame" ? "视频首帧" : "视频尾帧",
    });
  return {
    inputs,
    mode: inputs.some((r) => r.role === "last-frame") ? "ends" : "single",
    error: undefined,
  };
}
// Selecting a model never assigns frames by attachment order.
export function selectMediaModel(
  task: ProductionTask,
  model: MediaModel,
): Partial<ProductionTask> {
  const roles = frameRoles(model);
  const inputs = task.inputs;
  return {
    modelId: model.id,
    parameters: {},
    inputs,
    mode: roles.length
      ? inputs.some((r) => r.role === "last-frame")
        ? "ends"
        : "single"
      : "multi",
  };
}
