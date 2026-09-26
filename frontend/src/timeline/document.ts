import {
  duration,
  makeClip,
  uid,
  type Asset,
  type Clip,
  type Project,
  type Track,
} from "../model";
export const tracksOf = (p: Project) => p.tracks;
export const endTime = (p: Project) =>
  Math.max(
    0,
    ...p.clips.map((c) => (c.start ?? 0) + duration(c)),
    ...(p.captions ?? []).map((c) => c.end),
  );
export function appendAsset(
  p: Project,
  asset: Asset,
  at?: number,
  trackId?: string,
  shotId?: string,
): Project {
  if (asset.kind === "text" || asset.kind === "document") return p;
  const kind = asset.kind === "audio" ? "audio" : "video";
  const track =
    tracksOf(p).find((t) => t.id === trackId && t.kind === kind) ??
    tracksOf(p).find((t) => t.kind === kind);
  if (!track) return p;
  const start =
    at ??
    Math.max(
      0,
      ...p.clips
        .filter((c) => c.trackId === track.id)
        .map((c) => (c.start ?? 0) + duration(c)),
    );
  return {
    ...p,
    clips: [
      ...p.clips,
      { ...makeClip(asset), start, trackId: track.id, shotId },
    ],
  };
}
export function addTrack(p: Project, kind: Track["kind"]) {
  const tracks = tracksOf(p),
    id = uid();
  return {
    ...p,
    tracks: [
      ...tracks,
      {
        id,
        kind,
        name: `${kind === "video" ? "画面" : "音频"} ${tracks.filter((t) => t.kind === kind).length + 1}`,
      },
    ],
  };
}
export function validateClip(c: Clip, asset: Asset) {
  return (
    [
      c.trimIn,
      c.trimOut,
      c.speed,
      c.volume,
      c.start ?? 0,
      c.scale ?? 1,
      c.opacity ?? 1,
      c.x ?? 0.5,
      c.y ?? 0.5,
      c.fadeIn ?? 0,
      c.fadeOut ?? 0,
    ].every(Number.isFinite) &&
    c.trimIn >= 0 &&
    c.trimOut > c.trimIn &&
    (asset.kind === "image" || c.trimOut <= asset.duration + 0.001) &&
    c.speed >= 0.25 &&
    c.speed <= 4 &&
    c.volume >= 0 &&
    c.volume <= 1 &&
    (c.start ?? 0) >= 0 &&
    (c.scale ?? 1) >= 0.05 &&
    (c.scale ?? 1) <= 4 &&
    (c.opacity ?? 1) >= 0 &&
    (c.opacity ?? 1) <= 1 &&
    (c.x ?? 0.5) >= -1 &&
    (c.x ?? 0.5) <= 2 &&
    (c.y ?? 0.5) >= -1 &&
    (c.y ?? 0.5) <= 2 &&
    (c.fadeIn ?? 0) >= 0 &&
    (c.fadeOut ?? 0) >= 0 &&
    duration(c) <= 3600
  );
}
export function moveClip(
  p: Project,
  id: string,
  start: number,
  trackId: string,
) {
  const clip = p.clips.find((c) => c.id === id),
    target = tracksOf(p).find((t) => t.id === trackId);
  const asset = p.assets.find((a) => a.id === clip?.assetId);
  if (
    !clip ||
    !asset ||
    !target ||
    (asset.kind === "audio" && target.kind !== "audio")
  )
    return p;
  if (target.kind === "audio" && !asset.hasAudio) return p;
  return {
    ...p,
    clips: p.clips.map((c) =>
      c.id === id
        ? {
            ...c,
            start: Math.max(0, Math.round(start * p.fps) / p.fps),
            trackId,
          }
        : c,
    ),
  };
}
