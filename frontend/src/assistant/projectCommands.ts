import { patchVisual } from "../timeline/visualSettings";
import { editTaskPrompt, regenerationDraft } from "../production/taskEditing";
import { saveTask } from "../production/document";
import { creativeExtras } from "../creative/operations";
import { assemblePlan, chooseTake } from "../creative/timeline";
import { appendAsset, tracksOf, validateClip } from "../timeline/document";
import { creationOperation, nodeExtras } from "./creationOperations";
import { type BoardNode, type Clip, type Project } from "../model";
import { removeNodes } from "../canvas/removeNodes";
import {
  requestTask,
  type GenerationCommandContext,
} from "../production/requestTask";
type Operation = Record<string, unknown>;
function string(v: unknown, max = 24000): string {
  if (typeof v !== "string" || v.length > max)
    throw new Error("文字字段无效或过长");
  return v;
}
function number(v: unknown, fallback: number): number {
  if (v === undefined) return fallback;
  if (typeof v !== "number" || !Number.isFinite(v))
    throw new Error("数值字段无效");
  return v;
}
function clipPatch(clip: Clip, op: Operation, p: Project) {
  const next = {
    ...clip,
    ...Object.fromEntries(
      [
        "trimIn",
        "trimOut",
        "speed",
        "volume",
        "start",
        "x",
        "y",
        "scale",
        "opacity",
        "fadeIn",
        "fadeOut",
      ]
        .filter((k) => op[k] !== undefined)
        .map((k) => [k, number(op[k], 0)]),
    ),
  };
  if (op.trackId !== undefined) next.trackId = string(op.trackId, 100);
  if (op.visual !== undefined)
    next.visual = patchVisual(clip.visual, op.visual);
  const asset = p.assets.find((a) => a.id === next.assetId);
  const track = tracksOf(p).find((t) => t.id === next.trackId);
  if (
    !asset ||
    !track ||
    !validateClip(next, asset) ||
    (op.visual !== undefined && track.kind !== "video") ||
    (track.kind === "video" ? asset.kind === "audio" : !asset.hasAudio)
  )
    throw new Error("片段参数或轨道无效");
  return next;
}
export function applyOperations(
  p: Project,
  revision: number,
  raw: unknown,
  context?: GenerationCommandContext,
): Project {
  if (revision !== (p.revision || 0))
    throw new Error("工程已变化，请重新 inspect 后使用最新 revision");
  if (
    !Array.isArray(raw) ||
    !raw.length ||
    raw.length > 30 ||
    JSON.stringify(raw).length > 100000
  )
    throw new Error("每次操作应为 1–30 项，总内容不超过 100 KB");
  let next = p;
  for (const value of raw) {
    if (!value || typeof value !== "object" || Array.isArray(value))
      throw new Error("操作格式无效");
    const op = value as Operation;
    switch (op.op) {
      case "add_node": {
        const id = string(op.id, 80),
          kind = string(op.kind, 20) as BoardNode["kind"];
        if (!/^[\w-]+$/.test(id) || next.nodes.some((n) => n.id === id))
          throw new Error("节点 ID 已存在或格式无效");
        if (!["text", "shot", "note", "asset", "plan"].includes(kind))
          throw new Error("节点类型无效");
        if (kind === "plan" && next.nodes.some((n) => n.kind === "plan"))
          throw new Error(
            "项目已有脚本，请使用 update_node 修改现有脚本并保留 ID",
          );
        const assetId =
          op.assetId === undefined ? undefined : string(op.assetId, 100);
        if (kind === "asset" && !next.assets.some((a) => a.id === assetId))
          throw new Error("只能关联本项目已有素材");
        const node: BoardNode = {
          id,
          kind,
          title: string(op.title, 300),
          text: op.text === undefined ? "" : string(op.text),
          assetId,
          x: number(
            op.x,
            kind === "plan"
              ? (32 - next.viewport.x) / next.viewport.scale
              : 400 + (next.nodes.length % 3) * 360,
          ),
          y: number(
            op.y,
            kind === "plan"
              ? (94 - next.viewport.y) / next.viewport.scale
              : Math.floor(next.nodes.length / 3) * 300,
          ),
          width: kind === "plan" ? 780 : kind === "text" ? 360 : 280,
          height: kind === "plan" ? 640 : kind === "text" ? 300 : 218,
        };
        next = {
          ...next,
          nodes: [
            ...next.nodes,
            creativeExtras(next, nodeExtras(next, node, op), op),
          ],
        };
        break;
      }
      case "update_node": {
        const id = string(op.id, 80),
          node = next.nodes.find((n) => n.id === id);
        if (!node) throw new Error("节点不存在");
        const updated = {
          ...node,
          title: op.title === undefined ? node.title : string(op.title, 300),
          text: op.text === undefined ? node.text : string(op.text),
        };
        next = {
          ...next,
          nodes: next.nodes.map((n) =>
            n.id === id
              ? creativeExtras(next, nodeExtras(next, updated, op), op)
              : n,
          ),
        };
        if (
          op.resultAssetId !== undefined &&
          next.nodes.find((n) => n.id === id)?.shot
        )
          next = chooseTake(next, id, string(op.resultAssetId, 100));
        break;
      }
      case "choose_take":
        next = chooseTake(next, string(op.id, 80), string(op.assetId, 100));
        break;
      case "assemble_plan":
        next = assemblePlan(next, string(op.id, 80));
        break;
      case "update_generation":
        next = editTaskPrompt(
          next,
          string(op.taskKey, 300),
          string(op.text, 12000),
        );
        break;
      case "regenerate_generation": {
        if (!context || context.turn.projectId !== next.id)
          throw new Error("本轮生成上下文已结束，请重新发送要求");
        const key = `retry:${context.callId}:${raw.indexOf(value)}`;
        if (next.production?.drafts?.[key]) break;
        const source = next.production?.drafts?.[string(op.taskKey, 300)];
        if (!source) throw new Error("任务不存在");
        const task = regenerationDraft(
          source,
          `${context.callId}:${raw.indexOf(value)}`,
        );
        task.turnId = context.turnId;
        task.instruction = context.turn.instruction;
        task.status =
          context.turn.models.execution === "automatic" && task.modelId
            ? "READY"
            : "AWAITING_CONFIRMATION";
        next = saveTask(next, task);
        break;
      }
      case "request_generation":
        next = requestTask(next, op, context, raw.indexOf(value));
        break;
      case "remove_node":
        next = removeNodes(next, [string(op.id, 80)]);
        break;
      case "set_brief":
        next = { ...next, brief: string(op.text) };
        break;
      case "append_clip": {
        const asset = next.assets.find((a) => a.id === op.assetId);
        if (!asset) throw new Error("素材不存在");
        next = appendAsset(next, asset);
        const clip = clipPatch(next.clips.at(-1)!, op, next);
        next = {
          ...next,
          clips: next.clips.map((c) => (c.id === clip.id ? clip : c)),
        };
        break;
      }
      case "update_clip": {
        const c = next.clips.find((c) => c.id === op.id);
        if (!c) throw new Error("片段不存在");
        const clip = clipPatch(c, op, next);
        next = {
          ...next,
          clips: next.clips.map((c) => (c.id === clip.id ? clip : c)),
        };
        break;
      }
      default: {
        const updated = creationOperation(next, op);
        if (!updated) throw new Error("不支持的工程操作");
        next = updated;
      }
    }
  }
  if (next.nodes.length > 5000) throw new Error("节点数量超过上限");
  return next;
}
export { inspectProject } from "./inspectProject";
