import { removeNodes } from "../canvas/removeNodes";
import type { Project } from "../model";
import type { ProductionTask } from "../production/types";
import { itemKey } from "../production/types";

// Tombstones contain IDs only. They prevent stale saves, undo and job recovery
// from resurrecting files that have been permanently deleted.
export function deleteAssets(p: Project, ids: string[]): Project {
  const removed = new Set([...(p.removedAssetIds ?? []), ...ids]);
  if (!removed.size) return p;
  const gone = (id?: string) => !!id && removed.has(id);
  const nodeIds = new Set(
    p.nodes.filter((n) => gone(n.assetId)).map((n) => n.id),
  );
  const removeKey = (key: string) => {
    const [kind, id, asset] = key.split(":").map(decodeURIComponent);
    return kind === "node" ? nodeIds.has(id) : gone(asset);
  };
  const cleanTask = (task: ProductionTask): ProductionTask => {
    const inputs = task.inputs.filter(
      (r) => !gone(r.assetId) && !nodeIds.has(r.nodeId ?? ""),
    );
    const invalid = inputs.length !== task.inputs.length;
    return {
      ...task,
      inputs,
      resultAssetId: gone(task.resultAssetId) ? undefined : task.resultAssetId,
      resultAssetIds: task.resultAssetIds?.filter((id) => !gone(id)),
      ...(invalid &&
      ["READY", "AWAITING_CONFIRMATION"].includes(task.status ?? "")
        ? { status: "FAILED", error: "参考素材已删除，请重新选择后生成" }
        : {}),
    };
  };
  return {
    ...p,
    removedAssetIds: [...removed],
    assets: p.assets.filter((a) => !gone(a.id)),
    nodes: p.nodes
      .filter((n) => !nodeIds.has(n.id))
      .map((n) => ({
        ...n,
        resultAssetId: gone(n.resultAssetId) ? undefined : n.resultAssetId,
        references: n.references?.filter((r) => !gone(r.assetId)),
        shot: n.shot
          ? {
              ...n.shot,
              frames: n.shot.frames?.filter((f) => !gone(f.assetId)),
              takes: n.shot.takes
                ?.filter((t) => !gone(t.assetId))
                .map((t) => ({
                  ...t,
                  production: t.production
                    ? cleanTask(t.production)
                    : undefined,
                })),
            }
          : undefined,
      })),
    clips: p.clips.filter((c) => !gone(c.assetId)),
    captions: p.captions.filter((c) => !gone(c.assetId)),
    production: p.production
      ? {
          ...p.production,
          hidden: p.production.hidden?.filter((k) => !removeKey(k)),
          positions: p.production.positions
            ? Object.fromEntries(
                Object.entries(p.production.positions).filter(
                  ([k]) => !removeKey(k),
                ),
              )
            : undefined,
          drafts: p.production.drafts
            ? Object.fromEntries(
                Object.entries(p.production.drafts).map(([k, t]) => [
                  k,
                  cleanTask(t),
                ]),
              )
            : undefined,
        }
      : undefined,
  };
}

// Older versions called this action Delete but only hid the card. Complete
// those deletions when opening the project; task-list visibility is unrelated.
export function deleteLegacyHiddenAssets(p: Project): Project {
  const hidden = new Set(p.production?.hidden ?? []);
  const ids = p.nodes
    .filter((n) => n.assetId && hidden.has(itemKey("node", n.id)))
    .map((n) => n.assetId!);
  for (const node of p.nodes) {
    for (const [kind, media] of [
      ["frame", node.shot?.frames],
      ["take", node.shot?.takes],
    ] as const)
      for (const item of media ?? [])
        if (hidden.has(itemKey(kind, node.id, item.assetId)))
          ids.push(item.assetId);
  }
  return ids.length ? deleteAssets(p, ids) : p;
}

export function deleteCanvasNodes(p: Project, ids: string[]): Project {
  const assets = p.nodes
    .filter((n) => ids.includes(n.id) && n.assetId)
    .map((n) => n.assetId!);
  return removeNodes(deleteAssets(p, assets), ids);
}
