import { checkedFrames } from "./checkedFrames";
import { writeScript } from "./scriptWrite";
import { paragraphBasis } from "./script";
import type { BoardNode, Project } from "../model";
import { shotBasis, addShotResult } from "./document";
import { promptBasis } from "./prompt";
type Op = Record<string, unknown>;
const object = (v: unknown): Op => {
  if (!v || typeof v !== "object" || Array.isArray(v))
    throw new Error("方案或镜头格式无效");
  return v as Op;
};
const text = (v: unknown, max = 6000) => {
  if (typeof v !== "string" || v.length > max)
    throw new Error("方案文字无效或过长");
  return v;
};
export function creativeExtras(p: Project, node: BoardNode, op: Op): BoardNode {
  const previous = p.nodes.find((n) => n.id === node.id);
  if (op.plan !== undefined) {
    if (node.kind !== "plan") throw new Error("只有视频方案能设置故事结构");
    const v = object(op.plan);
    node = {
      ...node,
      plan: {
        ...node.plan,
        ...([
          v.script,
          v.scriptMode,
          v.removeParagraphIds,
          v.paragraphOrder,
        ].every((value) => value === undefined)
          ? {}
          : { script: writeScript(node.plan?.script, v) }),
        story: v.story === undefined ? (node.plan?.story ?? "") : text(v.story),
        sound: v.sound === undefined ? (node.plan?.sound ?? "") : text(v.sound),
      },
    };
  }
  if (op.shot !== undefined) {
    if (node.kind !== "shot") throw new Error("只有镜头能设置分镜内容");
    const v = object(op.shot),
      old = node.shot;
    for (const key of Object.keys(v))
      if (
        ![
          "planId",
          "scriptId",
          "order",
          "duration",
          "dialogue",
          "frames",
          "framePrompt",
          "prompt",
        ].includes(key)
      )
        throw new Error(`不支持的镜头字段：${key}`);
    const planId =
      v.planId === undefined && old ? old.planId : text(v.planId, 80);
    if (!p.nodes.some((n) => n.id === planId && n.kind === "plan"))
      throw new Error("请先创建视频方案，再添加对应镜头");
    const scriptId =
      v.scriptId === undefined ? old?.scriptId : text(v.scriptId, 100);
    const source = p.nodes
      .find((n) => n.id === planId)
      ?.plan?.script?.find((s) => s.id === scriptId);
    if (scriptId && !source) throw new Error("脚本段落不属于当前方案");
    const order = v.order ?? old?.order,
      duration = v.duration ?? old?.duration;
    if (
      typeof order !== "number" ||
      !Number.isInteger(order) ||
      order < 1 ||
      order > 999 ||
      typeof duration !== "number" ||
      !Number.isFinite(duration) ||
      duration <= 0 ||
      duration > 3600
    )
      throw new Error("镜头顺序或时长无效");
    node = {
      ...node,
      shot: {
        ...old,
        planId,
        scriptId,
        scriptBasis:
          source && (!old || old.scriptId !== scriptId)
            ? paragraphBasis(source)
            : old?.scriptBasis,
        order,
        duration,
        frames:
          v.frames === undefined ? old?.frames : checkedFrames(p, v.frames),
        framePrompt:
          v.framePrompt === undefined
            ? old?.framePrompt
            : text(v.framePrompt, 12000),
        dialogue:
          v.dialogue === undefined ? (old?.dialogue ?? "") : text(v.dialogue),
        prompt: v.prompt === undefined ? old?.prompt : text(v.prompt, 12000),
      },
    };
    if (v.prompt !== undefined)
      node = {
        ...node,
        shot: { ...node.shot!, promptBasis: promptBasis(p, node) },
      };
  }
  if (node.kind === "shot" && !node.shot)
    throw new Error("镜头必须属于一个视频方案");
  if (node.kind === "plan" && !node.plan)
    node = { ...node, plan: { story: "", sound: "" } };
  if (node.shot && previous && shotBasis(previous) !== shotBasis(node)) {
    node = {
      ...node,
      shot: {
        ...node.shot,
        visualChanged: true,
      },
    };
  }
  if (node.shot && op.resultAssetId !== undefined) {
    const asset = p.assets.find((a) => a.id === op.resultAssetId)!;
    node = addShotResult({ ...p, nodes: [node] }, asset, node.id).nodes[0];
  }
  return node;
}

// Order is a batch invariant: swaps may temporarily occupy the same slot.
export function validateShotOrder(p: Project) {
  const seen = new Set<string>();
  for (const node of p.nodes) {
    if (!node.shot) continue;
    const key = `${node.shot.planId}:${node.shot.order}`;
    if (seen.has(key)) throw new Error("方案中已有相同顺序的镜头");
    seen.add(key);
  }
}
