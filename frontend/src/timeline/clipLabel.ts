import { t } from "../i18n";
import type { Asset, Clip, Project } from "../model";
import { taskTitle } from "../production/taskTitle";

/** Presentation only: file names and media references remain unchanged. */
export function clipLabel(project: Project, clip: Clip, asset?: Asset): string {
  if (!asset) return t("素材丢失");
  if (!asset.generated) return asset.name;
  const shot = project.nodes.find((node) => node.id === clip.shotId);
  if (shot?.title) return shot.title;
  const task = Object.values(project.production?.drafts ?? {}).find(
    (task) =>
      task.resultAssetId === asset.id ||
      task.resultAssetIds?.includes(asset.id),
  );
  return task ? taskTitle(task, t("未命名")) : asset.name;
}
