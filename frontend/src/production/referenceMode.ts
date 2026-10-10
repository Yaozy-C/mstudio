import { t } from "../i18n";
import { mediaAdapter } from "../models/adapters";
import type { MediaModel } from "../models/mediaRegistry";
import type { ProductionTask } from "./types";

export type ComposerMode = "agent" | "image" | "video" | "reference";
export const isVideoMode = (mode: ComposerMode) =>
  mode === "video" || mode === "reference";

export function supportsVideoReference(model: MediaModel) {
  return (
    model.kind === "video" &&
    mediaAdapter(model).fields.some(
      (f) => f.kind === "video" && f.role === "reference",
    )
  );
}

export function referenceIssue(task: ProductionTask, model?: MediaModel) {
  if (model && !supportsVideoReference(model))
    return t("请选择支持视频参考的生成模型");
  if (task.inputs.some((r) => r.role === "script"))
    return t("参考模式仅使用图片和视频，请移除文字资料并将要求写入描述");
  if (
    task.kind !== "video" ||
    !task.inputs.some((r) => r.role === "video-reference")
  )
    return t("请添加一段参考视频");
  return "";
}
