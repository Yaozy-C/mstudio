import {
  duration,
  uid,
  type BoardNode,
  type Clip,
  type Project,
} from "../model";
import { appendAsset, tracksOf, validateClip } from "../timeline/document";
import { shotsOf, shotBasis, takesOf } from "./document";
import type { ShotTake } from "./types";
function checkedTake(p: Project, n: BoardNode, assetId: string): ShotTake {
  const take = takesOf(n).find((t) => t.assetId === assetId);
  const asset = p.assets.find((a) => a.id === assetId && a.kind !== "audio");
  if (
    !take ||
    !asset ||
    !validateClip(
      {
        id: "check",
        assetId,
        trimIn: take.trimIn,
        trimOut: take.trimOut,
        speed: 1,
        volume: 0,
        start: 0,
        trackId: "v1",
      },
      asset,
    )
  )
    throw new Error("镜头结果丢失或区间无效");
  return take;
}
export function chooseTake(
  project: Project,
  shotId: string,
  assetId: string,
): Project {
  const p = project,
    node = p.nodes.find((n) => n.id === shotId);
  if (!node?.shot) throw new Error("镜头不存在");
  const take = checkedTake(p, node, assetId);
  const visualTracks = new Set(
    tracksOf(p)
      .filter((t) => t.kind === "video")
      .map((t) => t.id),
  );
  const affected = p.clips.filter(
    (c) => c.shotId === shotId && visualTracks.has(c.trackId!),
  );
  const changing = affected.filter((c) => c.assetId !== assetId);
  let clips = p.clips,
    tracks = tracksOf(p);
  if (changing.length) {
    // Preserve all cut positions, split boundaries and overlays; only map the source range.
    const start = Math.min(...affected.map((c) => c.start ?? 0));
    const end = Math.max(...affected.map((c) => (c.start ?? 0) + duration(c)));
    const speed = (take.trimOut - take.trimIn) / (end - start);
    if (speed < 0.25 || speed > 4)
      throw new Error("候选时长与当前镜头差距过大，请先调整素材区间");
    const audioId = uid(),
      preserved: Clip[] = [];
    const ids = new Set(changing.map((c) => c.id));
    const asset = p.assets.find((a) => a.id === assetId)!;
    clips = clips.map((c) => {
      if (!ids.has(c.id)) return c;
      const old = p.assets.find((a) => a.id === c.assetId);
      if (old?.hasAudio && c.volume > 0)
        preserved.push({
          ...c,
          id: uid(),
          trackId: audioId,
          shotId: undefined,
        });
      const trimIn = take.trimIn + ((c.start ?? 0) - start) * speed;
      const next = {
        ...c,
        assetId,
        trimIn,
        trimOut: trimIn + duration(c) * speed,
        speed,
        volume: 0,
      };
      if (!validateClip(next, asset)) throw new Error("替换后片段超出素材范围");
      return next;
    });
    if (preserved.length) {
      clips = [...clips, ...preserved];
      tracks = [
        ...tracks,
        { id: audioId, name: `${node.title} · 原声音`, kind: "audio" },
      ];
    }
  }
  return {
    ...p,
    clips,
    tracks,
    nodes: p.nodes.map((n) =>
      n.id === shotId
        ? {
            ...n,
            resultAssetId: assetId,
            shot: {
              ...node.shot!,
              visualChanged: take.basis
                ? take.basis !== shotBasis(node)
                : node.shot!.visualChanged,
            },
          }
        : n,
    ),
  };
}
export function assembleScreenplay(
  project: Project,
  screenplayId: string,
): Project {
  let p = project;
  const shots = shotsOf(p, screenplayId);
  if (!shots.length) throw new Error("脚本还没有镜头");
  if (shots.some((s) => !s.resultAssetId))
    throw new Error("先为每个镜头选用画面，再编排到时间线");
  const visualTracks = new Set(
    tracksOf(p)
      .filter((t) => t.kind === "video")
      .map((t) => t.id),
  );
  const missing = shots.filter(
    (n) =>
      !p.clips.some((c) => c.shotId === n.id && visualTracks.has(c.trackId!)),
  );
  if (!missing.length) return project;
  const trackId = uid();
  p = {
    ...p,
    tracks: [...tracksOf(p), { id: trackId, kind: "video", name: "分镜画面" }],
  };
  let start = 0;
  for (const n of shots) {
    if (!missing.includes(n)) {
      const existing = p.clips.filter(
        (c) => c.shotId === n.id && visualTracks.has(c.trackId!),
      );
      start = Math.max(
        start,
        ...existing.map((c) => (c.start ?? 0) + duration(c)),
      );
      continue;
    }
    const asset = p.assets.find((a) => a.id === n.resultAssetId)!;
    const take = checkedTake(p, n, n.resultAssetId!);
    const speed = (take.trimOut - take.trimIn) / n.shot!.duration;
    if (speed < 0.25 || speed > 4)
      throw new Error(`「${n.title}」素材与计划时长不匹配，请先调整镜头时长`);
    p = appendAsset(p, asset, start, trackId, n.id);
    p = {
      ...p,
      clips: p.clips.map((c) =>
        c.id === p.clips.at(-1)!.id
          ? { ...c, trimIn: take.trimIn, trimOut: take.trimOut, speed }
          : c,
      ),
    };
    start += n.shot!.duration;
  }
  return p;
}
