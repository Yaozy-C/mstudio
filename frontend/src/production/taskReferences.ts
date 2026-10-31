import { referenceAsset } from "../previs/media";
import { failure } from "../errors/failure";
import type { Project } from "../model";
import { attachmentInput } from "./attachmentInput";
import type { ProductionInput, ProductionTask } from "./types";
import {
  canUseTaskReference,
  MATERIAL_REFERENCE_ERROR,
} from "../workspace/resourceRoles";

export function replaceTaskReferences(
  p: Project,
  source: ProductionTask,
  references: unknown,
): ProductionTask {
  const task = { ...source };
  const kind = task.kind;
  if (references !== undefined) {
    if (!Array.isArray(references))
      throw failure("VALIDATION_FAILED", "参考素材须为数组");
    const inputs = references.map((value): ProductionInput => {
      if (!value || typeof value !== "object")
        throw failure("VALIDATION_FAILED", "参考素材无效");
      const r = value as Record<string, unknown>;
      if (typeof r.assetId !== "string")
        throw failure("VALIDATION_FAILED", "缺少参考素材 ID");
      if (
        r.purpose !== undefined &&
        (typeof r.purpose !== "string" || [...r.purpose].length > 400)
      )
        throw failure(
          "VALIDATION_FAILED",
          "参考素材用途须为不超过 400 字的文本",
        );
      const input = attachmentInput(p, { kind: "asset", id: r.assetId });
      const asset = referenceAsset(p, r.assetId);
      if (!input || !asset || !["image", "video", "audio"].includes(asset.kind))
        throw failure(
          "VALIDATION_FAILED",
          "只能使用项目中已有的图片、视频或音频",
        );
      const role =
        r.role ?? (asset.kind === "video" ? "video-reference" : "reference");
      const allowed =
        asset.kind === "image"
          ? kind === "image"
            ? ["edit", "reference"]
            : ["reference", "first-frame", "last-frame"]
          : asset.kind === "video"
            ? ["video-reference"]
            : ["reference"];
      if (!allowed.includes(String(role)))
        throw failure("VALIDATION_FAILED", "素材用途与类型不匹配");
      const frame = p.nodes.find((n) =>
        n.shot?.frames?.some((f) => f.assetId === asset.id),
      );
      for (const field of ["start", "end"])
        if (
          r[field] !== undefined &&
          (typeof r[field] !== "number" || !Number.isFinite(r[field]))
        )
          throw failure("VALIDATION_FAILED", "视频区间必须是有效秒数");
      const start = (r.start as number | undefined) ?? input.start;
      const end = (r.end as number | undefined) ?? input.end;
      if (
        asset.kind === "video" &&
        (start === undefined ||
          end === undefined ||
          start < 0 ||
          end > asset.duration ||
          end <= start)
      )
        throw failure("VALIDATION_FAILED", "请指定有效的视频参考区间");
      return {
        ...input,
        key: `asset:${input.assetId}:${role}`,
        role: role as ProductionInput["role"],
        start,
        end,
        nodeId: frame?.id ?? input.nodeId,
        frame: !!frame,
        purpose:
          r.purpose === undefined ? input.purpose : (r.purpose as string),
      };
    });
    if (
      new Set(inputs.map((r) => `${r.assetId}:${r.role}`)).size !==
      inputs.length
    )
      throw failure("VALIDATION_FAILED", "参考素材不能重复");
    task.inputs = [
      ...task.inputs.filter((r) => r.role === "script"),
      ...inputs,
    ];
  }
  return task;
}

export function assertGenerationReferences(p: Project, task: ProductionTask) {
  for (const input of task.inputs.filter((r) => r.role !== "script")) {
    const asset = referenceAsset(p, input.assetId);
    if (asset?.annotationSourceId)
      throw failure(
        "VALIDATION_FAILED",
        "内部批注图不能用作生成参考，请使用原图",
      );
    if (!asset || asset.missing)
      throw failure(
        "VALIDATION_FAILED",
        "参考素材已移除或不可用，请替换该参考",
      );
    if (asset.inLibrary === false)
      throw failure(
        "VALIDATION_FAILED",
        "参考素材已从素材库移除，请移除该参考或先恢复素材",
      );
    if (!canUseTaskReference(p, asset, task))
      throw failure("VALIDATION_FAILED", MATERIAL_REFERENCE_ERROR);
  }
}
