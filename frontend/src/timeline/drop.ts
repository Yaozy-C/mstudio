import type { Project } from "../model";
import { appendAsset, moveClip } from "./document";

export const CLIP_DRAG_TYPE = "application/mstudio-clip";
export const REFERENCE_DRAG_TYPE = "application/x-mstudio-reference";

export function dropOnTimeline(
  project: Project,
  type: string,
  raw: string,
  time: number,
  trackId: string,
): Project {
  if (!Number.isFinite(time)) return project;
  try {
    const data = JSON.parse(raw);
    if (!data || typeof data.id !== "string") return project;
    if (type === CLIP_DRAG_TYPE) {
      if (typeof data.offset !== "number" || !Number.isFinite(data.offset))
        return project;
      return moveClip(project, data.id, time - data.offset, trackId);
    }
    if (type !== REFERENCE_DRAG_TYPE || data.kind !== "asset") return project;
    const asset = project.assets.find((a) => a.id === data.id);
    const track = project.tracks.find((t) => t.id === trackId);
    if (!asset || asset.missing || !track) return project;
    if (
      (asset.kind !== "image" &&
        asset.kind !== "video" &&
        asset.kind !== "audio") ||
      track.kind !== (asset.kind === "audio" ? "audio" : "video")
    )
      return project;
    return appendAsset(
      project,
      asset,
      Math.max(0, Math.round(time * project.fps) / project.fps),
      trackId,
    );
  } catch {
    return project;
  }
}
