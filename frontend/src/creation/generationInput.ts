import { referenceAsset } from "../previs/media";
import type { MediaModel } from "../models/mediaRegistry";
import { validateMediaModel } from "../models/validateMediaModel";
import {
  mediaAdapter,
  presetRequest,
  type ModelInput,
} from "../models/adapters";
import type { Project, Reference } from "../model";
import { referenceGroups } from "./referenceGroups";
export type UploadedReference = {
  assetId: string;
  kind: "image" | "video";
  url: string;
  role?: ModelInput["role"];
  sourceAssetIds?: string[];
};
export function registeredInput(
  model: MediaModel,
  project: Project,
  prompt: string,
  refs: Reference[],
  uploaded: (UploadedReference & { role: ModelInput["role"] })[],
) {
  if (!prompt.trim()) throw new Error("请输入生成描述");
  validateMediaModel(model);
  if (!model.enabled) throw new Error("模型已停用");
  const adapter = mediaAdapter(model);
  const request = presetRequest(
    adapter,
    requestPrompt(project, prompt, refs),
    model.params,
  );
  for (const [index, ref] of refs.entries()) {
    const asset = referenceAsset(project, ref.assetId);
    const file = uploaded[index];
    if (
      !asset ||
      !file ||
      file.assetId !== ref.assetId ||
      file.kind !== asset.kind
    )
      throw new Error("参考素材丢失或尚未上传");
    if (
      asset.kind === "video" &&
      (!Number.isFinite(ref.start ?? 0) ||
        !Number.isFinite(ref.end ?? asset.duration) ||
        (ref.start ?? 0) < 0 ||
        (ref.end ?? asset.duration) > asset.duration ||
        (ref.end ?? asset.duration) <= (ref.start ?? 0))
    )
      throw new Error("参考视频区间无效");
    const fields = adapter.fields.filter((f) => f.kind === file.kind);
    const field = fields.find((f) => f.role === file.role);
    if (!field)
      throw new Error("此模型不支持所选素材用途，请更换模型或调整素材用途");
  }
  for (const group of referenceGroups(project, refs)) {
    const file = uploaded[group[0]];
    if (group.some((index) => uploaded[index].url !== file.url))
      throw new Error("合并参考视频尚未上传");
    request.inputs!.push({
      kind: file.kind,
      role: file.role,
      url: file.url,
    });
  }
  return adapter.encode(request);
}

export function requestPrompt(
  project: Project,
  prompt: string,
  refs: Reference[],
) {
  let images = 0,
    videos = 0;
  const roles = referenceGroups(project, refs).flatMap((group) => {
    const kind = referenceAsset(project, refs[group[0]].assetId)?.kind;
    const label = kind === "image" ? `Image ${++images}` : `Video ${++videos}`;
    let offset = 0;
    return group.map((index) => {
      const r = refs[index];
      const asset = referenceAsset(project, r.assetId);
      const start = offset;
      offset += (r.end ?? asset?.duration ?? 0) - (r.start ?? 0);
      const range =
        group.length > 1 ? ` (${start.toFixed(3)}–${offset.toFixed(3)}s)` : "";
      return `${label}${range}: ${r.purpose || "按镜头描述使用此参考"}`;
    });
  });
  return [prompt, ...roles].filter(Boolean).join("\n\n");
}
