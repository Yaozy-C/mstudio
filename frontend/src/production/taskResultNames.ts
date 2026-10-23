import type { Project } from "../model";
import type { ProductionSource, ProductionTask } from "./types";
import { taskTitle } from "./taskTitle";

export function resultTask(
  p: Project,
  source: ProductionSource,
): ProductionTask {
  if (!source?.canvasGeneration)
    throw new Error("生成任务缺少制作上下文，未写入项目");
  const snapshot = source.canvasGeneration.task;
  return p.production?.drafts?.[snapshot.key] ?? snapshot;
}

export function resultTaskName(p: Project, source: ProductionSource): string {
  return taskTitle(resultTask(p, source));
}

/** A task's name owns its generated media's display metadata, never its file path. */
export function withTaskResultNames(p: Project, task: ProductionTask): Project {
  const ids = new Set(task.resultAssetIds ?? []);
  if (task.resultAssetId) ids.add(task.resultAssetId);
  if (!ids.size) return p;
  const title = taskTitle(task);
  const assets = p.assets.map((asset) =>
    ids.has(asset.id) && asset.name !== title
      ? { ...asset, name: title }
      : asset,
  );
  const nodes = p.nodes.map((node) =>
    node.assetId && ids.has(node.assetId) && node.title !== title
      ? { ...node, title }
      : node,
  );
  return assets.some((asset, i) => asset !== p.assets[i]) ||
    nodes.some((node, i) => node !== p.nodes[i])
    ? { ...p, assets, nodes }
    : p;
}
