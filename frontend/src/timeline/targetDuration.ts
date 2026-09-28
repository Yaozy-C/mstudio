import type { Clip } from "../model";

export function targetDurationRange(clip: Clip) {
  const source = clip.trimOut - clip.trimIn;
  return {
    min: Math.max(0.1, Math.ceil((source / 4 - 1e-9) * 10) / 10),
    max: Math.floor((Math.min(3600, source / 0.25) + 1e-9) * 10) / 10,
  };
}

export function speedForDuration(clip: Clip, seconds: number): number | null {
  const { min, max } = targetDurationRange(clip);
  if (
    !Number.isFinite(seconds) ||
    seconds < min ||
    seconds > max ||
    Math.abs(seconds * 10 - Math.round(seconds * 10)) > 1e-8
  )
    return null;
  return (clip.trimOut - clip.trimIn) / seconds;
}
