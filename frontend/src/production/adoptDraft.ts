import type { Project } from "../model";
import { addShotResult } from "../creative/document";

// Adoption is explicit. A scratch card remains editable after its result is used.
export function adoptDraft(
  p: Project,
  assetId: string,
  shotId: string,
): Project {
  const asset = p.assets.find((a) => a.id === assetId);
  const shot = p.nodes.find((n) => n.id === shotId && n.shot);
  if (!asset || asset.missing || !["image", "video"].includes(asset.kind))
    throw new Error("请选择可用的图片或视频");
  if (!shot?.shot) throw new Error("目标镜头不存在");
  if (asset.kind === "video") {
    if (shot.shot.takes?.some((t) => t.assetId === assetId)) return p;
    return addShotResult(p, asset, shotId);
  }
  if (shot.shot.frames?.some((f) => f.assetId === assetId)) return p;
  return {
    ...p,
    nodes: p.nodes.map((n) =>
      n.id === shotId
        ? {
            ...n,
            shot: {
              ...shot.shot!,
              frames: [
                ...(shot.shot!.frames ?? []),
                { assetId, title: asset.name },
              ],
            },
          }
        : n,
    ),
  };
}
