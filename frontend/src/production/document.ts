import { freeResultPosition } from "./resultLayout";
import { mediaCardSize } from "./mediaCardSize";
import {
  resultTask,
  resultTaskName,
  withTaskResultNames,
} from "./taskResultNames";
import { materializeFrameCards } from "./frameCards";
import { resultOrigin } from "./resultOrigin";
import { uid, type Asset, type Project } from "../model";
import { takesOf, shotBasis } from "../creative/document";
import { framesOf } from "./frames";
import { itemKey, type ProductionSource, type ProductionTask } from "./types";

export function saveTask(p: Project, task: ProductionTask): Project {
  p = withTaskResultNames(p, task);
  return {
    ...p,
    production: {
      ...p.production,
      drafts: {
        ...p.production?.drafts,
        [task.key]: task,
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
  if (!source?.canvasGeneration)
    throw new Error("生成任务缺少制作上下文，未写入项目");
  if (p.removedAssetIds?.includes(asset.id)) return p;
  const current = resultTask(p, source);
  asset = {
    ...asset,
    name: resultTaskName(p, source),
    generated: true,
    inLibrary: undefined,
  };
  const original = source.canvasGeneration;
  const ownerId =
    original.task.generationPurpose === "asset"
      ? undefined
      : resultOrigin(p, original.task).ownerId;
  const data = {
    ...original,
    task: { ...original.task, title: current.title, ownerId },
  };
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
  // Library membership and shot usage are independent of canvas visibility.
  // Reference generation creates a standalone card like other unassigned media.
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
  let key: string | undefined;
  const position = freeResultPosition(
    p,
    data.task,
    data.task.position ?? { x: data.x, y: data.y },
    mediaCardSize(asset),
  );
  let nodes = p.nodes;
  if (node?.shot) {
    if (asset.kind === "video") key = itemKey("take", node.id, asset.id);
    const shot =
      asset.kind === "image"
        ? {
            ...node.shot,
            frames: [
              ...framesOf(node),
              {
                assetId: asset.id,
                title: `画面 ${String(framesOf(node).length + 1).padStart(2, "0")}`,
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
        text: "",
        x: data.x,
        y: data.y,
      },
    ];
  }
  const result = materializeFrameCards({ ...p, assets, nodes });
  if (!key) {
    const card = result.nodes.find((n) => !n.shot && n.assetId === asset.id);
    if (!card) throw new Error("分镜图缺少画布卡片");
    key = itemKey("node", card.id);
  }
  return {
    ...result,
    production: {
      ...result.production,
      positions: {
        ...result.production?.positions,
        [key]: position,
      },
    },
  };
}
