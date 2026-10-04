import { DomainError } from "../domain/domainError";
import { checkedFrames } from "./checkedFrames";
import { writeScript } from "./scriptWrite";
import { paragraphBasis } from "./script";
import type { BoardNode, Project } from "../model";
import { shotBasis, addShotResult } from "./document";
import { promptBasis } from "./prompt";
type Op = Record<string, unknown>;
const object = (v: unknown): Op => {
  if (!v || typeof v !== "object" || Array.isArray(v))
    throw new Error("脚本或镜头格式无效");
  return v as Op;
};
const text = (v: unknown, max = 6000) => {
  if (typeof v !== "string" || v.length > max)
    throw new Error("脚本文字无效或过长");
  return v;
};
export function creativeExtras(p: Project, node: BoardNode, op: Op): BoardNode {
  const previous = p.nodes.find((n) => n.id === node.id);
  if (op.screenplay !== undefined) {
    if (node.kind !== "screenplay")
      throw new Error("只有脚本文档能设置脚本段落");
    const v = object(op.screenplay);
    for (const key of Object.keys(v))
      if (
        ![
          "script",
          "scriptMode",
          "removeParagraphIds",
          "paragraphOrder",
        ].includes(key)
      )
        throw new Error(`不支持的脚本字段：${key}`);
    node = {
      ...node,
      screenplay: {
        ...node.screenplay,
        ...([
          v.script,
          v.scriptMode,
          v.removeParagraphIds,
          v.paragraphOrder,
        ].every((value) => value === undefined)
          ? {}
          : { script: writeScript(node.screenplay?.script, v) }),
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
          "screenplayId",
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
    const screenplayId =
      v.screenplayId === undefined && old
        ? old.screenplayId
        : text(v.screenplayId, 80);
    if (!p.nodes.some((n) => n.id === screenplayId && n.kind === "screenplay"))
      throw new DomainError(
        "SCREENPLAY_NOT_FOUND",
        `Shot ${node.id} references absent screenplay ${screenplayId}.`,
        { nodeId: node.id, screenplayId },
      );
    const scriptId =
      v.scriptId === undefined ? old?.scriptId : text(v.scriptId, 100);
    const source = p.nodes
      .find((n) => n.id === screenplayId)
      ?.screenplay?.script?.find((s) => s.id === scriptId);
    if (scriptId && !source)
      throw new DomainError(
        "PARAGRAPH_NOT_FOUND",
        `Shot ${node.id} references paragraph ${scriptId}, which is absent from screenplay ${screenplayId}.`,
        { nodeId: node.id, screenplayId, paragraphId: scriptId },
      );
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
        screenplayId,
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
    throw new Error("镜头必须关联一个脚本文档");
  if (
    node.kind === "screenplay" &&
    typeof op.text === "string" &&
    op.text.trim()
  )
    throw new Error("脚本内容请写入 screenplay.script");
  if (node.kind === "screenplay" && !node.screenplay)
    node = { ...node, screenplay: { script: [] } };
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
    const key = `${node.shot.screenplayId}:${node.shot.order}`;
    if (seen.has(key)) throw new Error("脚本中已有相同顺序的镜头");
    seen.add(key);
  }
}
