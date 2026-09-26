import { duration, type Clip } from "../model";
// Keep the opposite edge and source interval fixed; only playback speed changes.
export function stretchClip(
  clip: Clip,
  delta: number,
  side: "left" | "right",
  fps: number,
): Clip {
  const source = clip.trimOut - clip.trimIn;
  const length = duration(clip);
  const end = clip.start + length;
  const minimum = Math.max(1 / fps, source / 4);
  const maximum = Math.min(source / 0.25, side === "left" ? end : 3600);
  const wanted = length + (side === "left" ? -delta : delta);
  const next = Math.max(minimum, Math.min(maximum, wanted));
  return {
    ...clip,
    start: side === "left" ? end - next : clip.start,
    speed: source / next,
  };
}
