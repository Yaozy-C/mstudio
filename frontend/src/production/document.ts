import { normalizeTaskPrompt } from "./taskPrompt";
import { materializeFrameCards } from "./frameCards";
import { resultOrigin } from "./resultOrigin";
import { uid, type Asset, type Project } from "../model";
import { takesOf, shotBasis } from "../creative/document";
import { framesOf } from "./frames";
import { productionItems, productionShots } from "./items";
import { itemKey, type ProductionSource, type ProductionTask } from "./types";

export function saveTask(p: Project, task: ProductionTask): Project {
  return {
    ...p,
    production: {
      ...p.production,
      drafts: {
        ...p.production?.drafts,
        [task.key]: normalizeTaskPrompt(task),
      },
    },
  };
}
export function jobStatus(
  p: Project,
  id: string,
  status: string,
  error?: string,
  progress?: ProductionTask["progress"],
  requestId?: string,
): Project {
  const drafts = p.production?.drafts;
  if (!drafts) return p;
  const task = Object.values(drafts).find(
    (t) => t.jobId === id || t.submissionId === id,
  );
  if (
    !task ||
    (task.status === status &&
      task.error === error &&
      (!requestId || task.requestId === requestId) &&
      JSON.stringify(task.progress) === JSON.stringify(progress))
  )
    return p;
  return saveTask(p, {
    ...task,
    jobId: id,
    status,
    error,
    progress,
    trackingPaused: error ? task.trackingPaused : false,
    requestId: requestId ?? task.requestId,
  });
}
export function receiveProductionResult(
  p: Project,
  asset: Asset,
  source: ProductionSource,
): Project {
  asset = {
    ...asset,
    generated: true,
    ...(source.canvasGeneration?.task.generationPurpose === "asset"
      ? { inLibrary: true }
      : {}),
  };
  const original = source.canvasGeneration!;
  const { ownerId } = resultOrigin(p, original.task);
  const data = { ...original, task: { ...original.task, ownerId } };
  if (asset.kind !== data.task.kind)
    throw new Error("返回的媒体类型与生成任务不一致");
  const run = p.production?.drafts?.[data.task.key];
  if (
    run?.turnId &&
    (run.jobId ?? run.submissionId) ===
      (data.task.jobId ?? data.task.submissionId) &&
    (run.resultAssetId !== asset.id || run.ownerId !== ownerId)
  )
    p = saveTask(p, {
      ...run,
      ownerId,
      resultAssetId: run.resultAssetId ?? asset.id,
      resultAssetIds: [
        ...new Set([
          ...(run.resultAssetIds ??
            (run.resultAssetId ? [run.resultAssetId] : [])),
          asset.id,
        ]),
      ],
    });
  // Reference assets live in project media. Linking them to shots must not
  // materialize a canvas node; users can still place them manually.
  if (data.task.generationPurpose === "asset")
    return {
      ...p,
      assets: p.assets.some((a) => a.id === asset.id)
        ? p.assets.map((a) =>
            a.id === asset.id ? { ...a, inLibrary: true } : a,
          )
        : [...p.assets, asset],
    };
  const node = p.nodes.find((n) => n.id === data.task.ownerId && n.shot);
  if (
    node &&
    (framesOf(node).some((f) => f.assetId === asset.id) ||
      takesOf(node).some((t) => t.assetId === asset.id))
  )
    return p;
  if (!node && p.nodes.some((n) => n.assetId === asset.id)) return p;
  const assets = p.assets.some((a) => a.id === asset.id)
    ? p.assets
    : [...p.assets, asset];
  let key: string;
  let position = { x: data.x, y: data.y };
  let nodes = p.nodes;
  if (node?.shot) {
    const x = productionShots(p).findIndex((n) => n.id === node.id) * 1600;
    const count =
      asset.kind === "image"
        ? framesOf(node).length
        : takesOf(node).filter(
            (t) => p.assets.find((a) => a.id === t.assetId)?.kind === "video",
          ).length;
    position = {
      x: x + (asset.kind === "image" ? 535 + (count % 2) * 290 : 1135),
      y: 110 + (asset.kind === "image" ? Math.floor(count / 2) : count) * 395,
    };
    key = itemKey(asset.kind === "image" ? "frame" : "take", node.id, asset.id);
    const shot =
      asset.kind === "image"
        ? {
            ...node.shot,
            frames: [
              ...framesOf(node),
              {
                assetId: asset.id,
                title: `画面 ${String(framesOf(node).length + 1).padStart(2, "0")}`,
                prompt: data.task.prompt,
              },
            ],
          }
        : {
            ...node.shot,
            takes: [
              ...takesOf(node),
              {
                assetId: asset.id,
                trimIn: 0,
                trimOut: asset.duration,
                basis: shotBasis(source),
                production: data.task,
              },
            ],
          };
    nodes = nodes.map((n) => (n.id === node.id ? { ...n, shot } : n));
  } else {
    const id = uid();
    key = itemKey("node", id);
    nodes = [
      ...nodes,
      {
        id,
        kind: "asset",
        assetId: asset.id,
        title: asset.name,
        text: data.task.prompt,
        x: data.x,
        y: data.y,
      },
    ];
  }
  if (data.task.position) position = { ...data.task.position };
  const occupied = productionItems(p).filter((n) => n.assetId !== asset.id);
  while (
    occupied.some(
      (n) =>
        n.x < position.x + 250 &&
        n.x + n.width > position.x &&
        n.y < position.y + 350 &&
        n.y + n.height > position.y,
    )
  )
    position.y += 395;
  return materializeFrameCards({
    ...p,
    assets,
    nodes,
    production: {
      ...p.production,
      positions: {
        ...p.production?.positions,
        [key]: position,
      },
    },
  });
}
