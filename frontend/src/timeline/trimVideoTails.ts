import { duration, type Clip, type Project } from "../model";
import { t } from "../i18n";

/** Trim picture EOF to complete output frames; ripple only the removed tail. */
export function trimVideoTails(
  project: Project,
  ends: Record<string, number>,
): Project {
  const updates = new Map<string, Clip>();
  const assets = new Map(project.assets.map((a) => [a.id, a]));
  for (const track of project.tracks.filter((t) => t.kind === "video")) {
    const clips = project.clips
      .filter((c) => c.trackId === track.id)
      .sort((a, b) => a.start - b.start);
    let shift = 0;
    for (const clip of clips) {
      const end = ends[clip.assetId];
      let trimOut = clip.trimOut;
      if (
        assets.get(clip.assetId)?.kind === "video" &&
        Number.isFinite(end) &&
        end > 0 &&
        clip.trimOut >= end - 1e-8
      ) {
        const frames = Math.floor(
          ((end - clip.trimIn) / clip.speed) * project.fps + 1e-7,
        );
        if (frames < 1) throw new Error(t("片段不足一帧，未裁切"));
        trimOut = Math.min(
          clip.trimOut,
          clip.trimIn + (frames / project.fps) * clip.speed,
        );
      }
      const removed = (clip.trimOut - trimOut) / clip.speed;
      if (
        removed > 1e-8 &&
        clips.some(
          (other) =>
            other.id !== clip.id &&
            other.start < clip.start + duration(clip) - 1e-8 &&
            other.start + duration(other) >
              clip.start + duration(clip) - removed + 1e-8,
        )
      )
        throw new Error(t("片尾存在重叠片段，请先整理接缝再裁末尾"));
      if (removed > 1e-8 || shift > 1e-8)
        updates.set(clip.id, {
          ...clip,
          trimOut,
          start: clip.start - shift,
        });
      shift += removed;
    }
  }
  if (!updates.size) return project;
  const clips = project.clips.map((c) => updates.get(c.id) ?? c);
  // Retain valid transitions; a shortened shot may no longer support its seam.
  return {
    ...project,
    clips: clips.map((c) => {
      if (!c.transition) return c;
      const left = clips.find((l) => l.id === c.transition!.fromClipId);
      return left &&
        left.trackId === c.trackId &&
        Math.abs(left.start + duration(left) - c.start) <= 0.5 / project.fps &&
        c.transition.duration <= Math.min(duration(left), duration(c))
        ? c
        : { ...c, transition: undefined };
    }),
  };
}
