import { t } from "../i18n";
import type { Asset, Clip, Project } from "../model";

/** Presentation only: file names and media references remain unchanged. */
export function clipLabel(project: Project, clip: Clip, asset?: Asset): string {
  if (!asset) return t("素材丢失");
  if (!asset.generated) return asset.name;
  const shot = project.nodes.find((node) => node.id === clip.shotId);
  if (shot?.title) return shot.title;
  const owners = project.nodes.filter((node) =>
    node.shot?.takes?.some((take) => take.assetId === asset.id),
  );
  if (owners.length === 1 && owners[0].title) return owners[0].title;
  const task = Object.values(project.production?.drafts ?? {}).find(
    (task) =>
      task.resultAssetId === asset.id ||
      task.resultAssetIds?.includes(asset.id),
  );
  if (task?.instruction?.trim()) return task.instruction;
  const index =
    project.assets
      .filter((a) => a.kind === asset.kind)
      .findIndex((a) => a.id === asset.id) + 1;
  return `${t(asset.kind === "video" ? "视频" : asset.kind === "audio" ? "音频" : "图片")} ${String(index).padStart(2, "0")}`;
}
