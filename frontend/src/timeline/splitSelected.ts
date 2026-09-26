import { duration, splitClip, type Project } from "../model";
export function splitSelected(
  p: Project,
  id: string | null,
  position: number,
): Project {
  const time = Math.round(position * p.fps) / p.fps;
  const active = p.clips.find(
    (c) => time >= (c.start ?? 0) && time < (c.start ?? 0) + duration(c),
  );
  const target = id || active?.id;
  return target ? { ...p, clips: splitClip(p.clips, target, time, p.fps) } : p;
}
