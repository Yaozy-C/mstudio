import { duration, type Clip, type Project } from "../model";
import { validateClip } from "./document";

type Operation = Record<string, unknown>;
function numeric(value: unknown, name: string): number {
  if (typeof value !== "number" || !Number.isFinite(value))
    throw new Error(`${name}必须为有限数值`);
  return value;
}

/** Explicit edit semantics shared by agent commands; never silently trim a source. */
export function agentTimelineEdit(p: Project, op: Operation): Project {
  const original = p.clips.find((c) => c.id === op.id);
  if (!original) throw new Error("片段不存在");
  for (const key of ["ripple", "allowOverlap"])
    if (op[key] !== undefined && typeof op[key] !== "boolean")
      throw new Error(`${key}必须为布尔值`);
  const snap = (t: number) => Math.round(t * p.fps) / p.fps;
  const end = original.start + duration(original);
  let edited: Clip = { ...original };
  let shift = 0;
  switch (op.op) {
    case "move_clip": {
      const start = numeric(op.start, "时间线位置");
      if (start < 0) throw new Error("时间线位置不能为负数");
      if (op.trackId !== undefined && typeof op.trackId !== "string")
        throw new Error("轨道 ID 无效");
      edited.start = snap(start);
      edited.trackId = (op.trackId as string | undefined) ?? original.trackId;
      break;
    }
    case "retime_clip":
      edited.speed = numeric(op.speed, "速度");
      if (op.ripple === true) shift = duration(edited) - duration(original);
      break;
    case "slip_clip": {
      const offset = numeric(op.sourceOffset, "源素材偏移秒数");
      edited.trimIn += offset;
      edited.trimOut += offset;
      break;
    }
    default:
      throw new Error("不支持的剪辑操作");
  }
  const asset = p.assets.find((a) => a.id === edited.assetId);
  const track = p.tracks.find((t) => t.id === edited.trackId);
  if (
    !asset ||
    !track ||
    !validateClip(edited, asset) ||
    (track.kind === "video" ? asset.kind === "audio" : !asset.hasAudio)
  )
    throw new Error("片段参数、源素材余量或目标轨道无效");
  if (
    shift !== 0 &&
    p.clips.some(
      (c) =>
        c.id !== original.id &&
        c.trackId === original.trackId &&
        c.start < end - 1e-8 &&
        c.start + duration(c) > end + 1e-8,
    )
  )
    throw new Error("变速片段尾部有重叠片段，无法安全顺移；请先整理接缝");
  let clips = p.clips.map((c) =>
    c.id === original.id
      ? edited
      : shift !== 0 && c.trackId === original.trackId && c.start >= end - 1e-8
        ? { ...c, start: c.start + shift }
        : c,
  );
  if (
    op.op !== "slip_clip" &&
    op.allowOverlap !== true &&
    clips.some(
      (c) =>
        c.id !== edited.id &&
        c.trackId === edited.trackId &&
        c.start < edited.start + duration(edited) - 1e-8 &&
        c.start + duration(c) > edited.start + 1e-8,
    )
  )
    throw new Error(
      "操作会产生片段重叠；变速可指定 ripple，明确需要叠加时指定 allowOverlap",
    );
  // Keep valid seams after a ripple; remove references invalidated by a move/retime.
  clips = clips.map((c) => {
    if (!c.transition) return c;
    const left = clips.find((a) => a.id === c.transition!.fromClipId);
    return left &&
      left.trackId === c.trackId &&
      Math.abs(left.start + duration(left) - c.start) <= 0.5 / p.fps &&
      c.transition.duration <= Math.min(duration(left), duration(c))
      ? c
      : { ...c, transition: undefined };
  });
  return { ...p, clips };
}
