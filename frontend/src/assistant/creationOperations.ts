import { uid, type Project, type Reference, type BoardNode } from "../model";
import { validCaption } from "../creation/captions";
import { tracksOf } from "../timeline/document";
type Op = Record<string, unknown>;
const text = (v: unknown, max = 6000): string => {
  if (typeof v !== "string" || v.length > max)
    throw new Error("文字字段无效或过长");
  return v;
};
const number = (v: unknown): number => {
  if (typeof v !== "number" || !Number.isFinite(v))
    throw new Error("数值字段无效");
  return v;
};
export function checkedReferences(
  p: Project,
  raw: unknown,
  existing: Reference[] = [],
): Reference[] {
  if (!Array.isArray(raw)) throw new Error("参考数量无效");
  const refs = raw.map((r: Record<string, unknown>) => {
    if (!r || typeof r !== "object") throw new Error("参考格式无效");
    const a = p.assets.find((a) => a.id === r.assetId);
    if (a?.inLibrary === false && !existing.some((ref) => ref.assetId === a.id))
      throw new Error("素材已从素材库移除，不能新增引用；请先恢复素材");
    if (!a || a.kind === "audio")
      throw new Error("只能选择项目中的图片或视频参考");
    const out: Reference = { assetId: a.id, purpose: text(r.purpose, 400) };
    if (a.kind === "video") {
      out.start = r.start === undefined ? 0 : number(r.start);
      out.end = r.end === undefined ? a.duration : number(r.end);
      if (out.start < 0 || out.end <= out.start || out.end > a.duration)
        throw new Error("视频参考区间无效");
    }
    return out;
  });
  if (new Set(refs.map((r) => r.assetId)).size !== refs.length)
    throw new Error("参考素材重复");
  return refs;
}
export function nodeExtras(p: Project, node: BoardNode, op: Op) {
  const previous = p.nodes.find((n) => n.id === node.id);
  for (const field of ["assetId", "resultAssetId"] as const) {
    if (
      op[field] !== undefined &&
      op[field] !== previous?.[field] &&
      p.assets.some((a) => a.id === op[field] && a.inLibrary === false)
    )
      throw new Error("素材已从素材库移除，不能新增引用；请先恢复素材");
  }
  if (op.assetId !== undefined) {
    const id = text(op.assetId, 100);
    if (node.kind !== "asset" || !p.assets.some((a) => a.id === id))
      throw new Error("素材记录只能关联本项目已有素材");
    node = { ...node, assetId: id };
  }
  if (op.references !== undefined)
    node = {
      ...node,
      references: checkedReferences(p, op.references, previous?.references),
    };
  if (op.resultAssetId !== undefined) {
    const id = text(op.resultAssetId, 100);
    if (!p.assets.some((a) => a.id === id))
      throw new Error("结果素材不属于项目");
    node = { ...node, resultAssetId: id };
  }
  return node;
}
export function creationOperation(p: Project, op: Op): Project | null {
  switch (op.op) {
    case "set_creation": {
      const creation = {
        ...(p.creation ?? {
          intent: p.brief,
          essential: "",
          preserve: "",
          stage: "planning" as const,
        }),
      };
      for (const key of ["intent", "essential", "preserve"] as const)
        if (op[key] !== undefined) creation[key] = text(op[key]);
      if (op.stage !== undefined) {
        if (!["planning", "production", "editing"].includes(String(op.stage)))
          throw new Error("创作阶段无效");
        creation.stage = op.stage as typeof creation.stage;
      }
      return { ...p, creation };
    }
    case "set_references": {
      if (!p.nodes.some((n) => n.id === op.id)) throw new Error("镜头不存在");
      const node = p.nodes.find((n) => n.id === op.id)!;
      const mode = op.referenceMode ?? "replace";
      if (!["replace", "upsert", "remove"].includes(String(mode)))
        throw new Error("参考编辑方式无效");
      let references: Reference[];
      if (mode === "remove") {
        if (
          !Array.isArray(op.assetIds) ||
          !op.assetIds.every((id) => typeof id === "string")
        )
          throw new Error("请指定要解除引用的素材 ID");
        const removed = new Set(op.assetIds);
        references = (node.references ?? []).filter(
          (r) => !removed.has(r.assetId),
        );
      } else {
        const raw =
          mode === "upsert" && Array.isArray(op.references)
            ? op.references.map((ref) =>
                ref && typeof ref === "object"
                  ? {
                      ...node.references?.find(
                        (old) => old.assetId === ref.assetId,
                      ),
                      ...ref,
                    }
                  : ref,
              )
            : op.references;
        const incoming = checkedReferences(p, raw, node.references);
        if (mode === "replace") references = incoming;
        else {
          const merged = new Map(
            (node.references ?? []).map((r) => [r.assetId, r]),
          );
          for (const ref of incoming)
            merged.set(ref.assetId, { ...merged.get(ref.assetId), ...ref });
          references = [...merged.values()];
        }
      }
      return {
        ...p,
        nodes: p.nodes.map((n) =>
          n.id === op.id
            ? {
                ...n,
                references,
                ...(n.shot
                  ? {
                      shot: {
                        ...n.shot,
                        visualChanged: true,
                      },
                    }
                  : {}),
              }
            : n,
        ),
      };
    }
    case "add_track": {
      const tracks = tracksOf(p),
        id = op.id === undefined ? uid() : text(op.id, 80);
      if (
        tracks.some((t) => t.id === id) ||
        !["video", "audio"].includes(String(op.trackKind))
      )
        throw new Error("轨道 ID 或类型无效");
      return {
        ...p,
        tracks: [
          ...tracks,
          {
            id,
            name: text(op.title, 100),
            kind: op.trackKind as "video" | "audio",
          },
        ],
      };
    }
    case "update_track": {
      const tracks = tracksOf(p);
      if (!tracks.some((t) => t.id === op.id)) throw new Error("轨道不存在");
      for (const key of ["muted", "hidden"])
        if (op[key] !== undefined && typeof op[key] !== "boolean")
          throw new Error("轨道开关无效");
      return {
        ...p,
        tracks: tracks.map((t) =>
          t.id === op.id
            ? {
                ...t,
                name: op.title === undefined ? t.name : text(op.title, 100),
                muted: (op.muted as boolean | undefined) ?? t.muted,
                hidden: (op.hidden as boolean | undefined) ?? t.hidden,
              }
            : t,
        ),
      };
    }
    case "add_caption":
    case "update_caption": {
      const old = p.captions?.find((c) => c.id === op.id);
      if (op.op === "update_caption" && !old) throw new Error("字幕不存在");
      if (op.op === "add_caption" && old) throw new Error("字幕 ID 重复");
      const caption = {
        id: old?.id ?? (op.id === undefined ? uid() : text(op.id, 80)),
        text: op.text === undefined && old ? old.text : text(op.text, 1000),
        start: op.start === undefined && old ? old.start : number(op.start),
        end: op.end === undefined && old ? old.end : number(op.end),
      };
      if (!validCaption(caption)) throw new Error("字幕文字或时间无效");
      return {
        ...p,
        captions: old
          ? p.captions!.map((c) => (c.id === old.id ? caption : c))
          : [...(p.captions ?? []), caption],
      };
    }
    case "remove_caption":
      return { ...p, captions: p.captions?.filter((c) => c.id !== op.id) };
    case "remove_clip":
      return { ...p, clips: p.clips.filter((c) => c.id !== op.id) };
    default:
      return null;
  }
}
