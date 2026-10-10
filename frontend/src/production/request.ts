import { failure } from "../errors/failure";
import { taskParameters } from "./parameters";
import type { Project, Reference } from "../model";
import type { MediaModel } from "../models/mediaRegistry";
import { mediaAdapter, type ModelInput } from "../models/adapters";
import { pointerDelete, referenceLimits } from "../models/capabilities";
import {
  registeredInput,
  requestPrompt,
  type UploadedReference,
} from "../creation/generationInput";
import { framesOf } from "./frames";
import type { CanvasGeneration, ProductionTask } from "./types";

export const roleLabels = {
  edit: "要修改的图",
  reference: "内容参考",
  "first-frame": "首帧",
  "last-frame": "尾帧",
  "video-reference": "动作参考",
  "video-edit": "修改原视频",
  script: "脚本依据",
};
export function mediaReferences(task: ProductionTask): Reference[] {
  return task.inputs
    .filter((r) => r.role !== "script")
    .map((r) => {
      const label = roleLabels[r.role];
      const purpose = r.purpose?.trim() ?? "";
      // Older drafts stored the role label itself, or nothing, as the purpose.
      const described =
        !purpose || purpose === label || purpose.startsWith(`${label} · `)
          ? purpose || label
          : `${label} · ${purpose}`;
      return {
        assetId: r.assetId,
        purpose: described,
        start: r.start,
        end: r.end,
      };
    });
}
export function generationPrompt(_p: Project, task: ProductionTask) {
  // Script references are Agent context. The reviewed task prompt is the model
  // instruction; appending that context here can render notes or future actions.
  return task.prompt.trim();
}

export function submittedPrompt(p: Project, task: ProductionTask) {
  return requestPrompt(p, generationPrompt(p, task), mediaReferences(task));
}
export function inputFor(
  p: Project,
  task: ProductionTask,
  model: MediaModel,
  uploaded?: UploadedReference[],
) {
  if (!task.prompt.trim())
    throw failure("VALIDATION_FAILED", "先说说这次想生成或修改什么");
  if (!model.enabled || model.kind !== task.kind)
    throw failure("VALIDATION_FAILED", "请选择对应类型的已启用模型");
  if (task.kind === "video" && task.inputs.some((r) => r.role === "edit"))
    throw failure(
      "VALIDATION_FAILED",
      "请将修改图片改为视频参考或明确指定首帧",
    );
  if (
    task.kind === "image" &&
    task.inputs.some((r) => ["first-frame", "last-frame"].includes(r.role))
  )
    throw failure(
      "VALIDATION_FAILED",
      "图片生成不能使用首尾帧，请将用途改为内容参考",
    );
  const adapter = mediaAdapter(model);
  const options = {
    ...model.params,
    ...taskParameters(model, task.parameters),
  };
  // Only the materials shown in this task may be sent. Model presets cannot add hidden references.
  for (const f of adapter.fields) pointerDelete(options, f.key);
  // Fixed native builders still recognize these fields when references are disabled.
  if (model.plugin === "gemini-native") pointerDelete(options, "/image_urls");
  if (model.plugin === "codex-image") pointerDelete(options, "/image");
  const limits = referenceLimits(model);
  const refs = mediaReferences(task);
  if (limits.count !== null && refs.length > limits.count)
    throw failure("VALIDATION_FAILED", `本次最多使用 ${limits.count} 个素材`);
  if (task.inputs.some((r) => r.role === "video-edit"))
    throw failure(
      "VALIDATION_FAILED",
      "当前接入的模型暂不支持直接修改原视频；可将视频设为动作参考，重新生成",
    );
  const visual = task.inputs.filter((r) => r.role !== "script");
  if (
    task.kind === "video" &&
    task.mode === "ends" &&
    (visual.filter((r) => r.role === "first-frame").length !== 1 ||
      visual.filter((r) => r.role === "last-frame").length !== 1)
  )
    throw failure("VALIDATION_FAILED", "首尾帧需要分别指定一张首帧和一张尾帧");
  let seconds = 0;
  const files = refs.map((r, index) => {
    const asset = p.assets.find((a) => a.id === r.assetId);
    if (!asset || !["image", "video"].includes(asset.kind))
      throw failure("VALIDATION_FAILED", "素材已移除，或不支持用于媒体生成");
    if (asset.kind === "video") {
      const start = r.start ?? 0,
        end = r.end ?? asset.duration;
      if (
        !Number.isFinite(start + end) ||
        start < 0 ||
        end > asset.duration ||
        end <= start
      )
        throw failure("VALIDATION_FAILED", "参考区间须有效且不能超出原视频");
      seconds += end - start;
    }
    const ref = visual[index];
    const role: ModelInput["role"] = ["first-frame", "last-frame"].includes(
      ref.role,
    )
      ? (ref.role as ModelInput["role"])
      : "reference";
    if (!adapter.fields.some((f) => f.kind === asset.kind && f.role === role))
      throw failure(
        "VALIDATION_FAILED",
        `此模型不支持「${roleLabels[ref.role]}」的${asset.kind === "video" ? "视频" : "图片"}输入，请更换模型或素材用途`,
      );
    const file = uploaded?.find((f) => f.assetId === r.assetId);
    if (uploaded && !file)
      throw failure("VALIDATION_FAILED", "参考素材上传不完整，请重试");
    return {
      assetId: r.assetId,
      kind: asset.kind as "image" | "video",
      role,
      url:
        file?.url ??
        (["gemini-native", "codex-image"].includes(model.plugin)
          ? "data:image/png;base64,YQ=="
          : "https://example.invalid/validate"),
    };
  });
  if (limits.seconds !== null && seconds > limits.seconds)
    throw failure(
      "VALIDATION_FAILED",
      `参考视频合计不超过 ${limits.seconds} 秒`,
    );
  return registeredInput(
    { ...model, params: options },
    p,
    generationPrompt(p, task),
    refs,
    files,
  );
}
export function canvasSnapshot(
  p: Project,
  task: ProductionTask,
  position: { x: number; y: number },
): CanvasGeneration {
  if (task.ownerId && !p.nodes.some((n) => n.id === task.ownerId && n.shot))
    throw failure("VALIDATION_FAILED", "目标镜头已移除，请重新选择");
  for (const ref of task.inputs.filter((r) => r.frame)) {
    const node = p.nodes.find((n) => n.id === ref.nodeId);
    if (
      !node ||
      !framesOf(node).some((f) => f.assetId === ref.assetId) ||
      !p.assets.some((a) => a.id === ref.assetId && a.kind === "image")
    )
      throw failure("VALIDATION_FAILED", "所选分镜图已移除");
  }
  return structuredClone({ task, ...position });
}
