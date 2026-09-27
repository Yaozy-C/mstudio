import { duration, uid, type Clip, type Project } from "../model";
export type ClipEdit = "copy" | "cut" | "paste" | "trim-start" | "trim-end";
export function editKey(
  e: Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey">,
): ClipEdit | undefined {
  const cmd = e.metaKey || e.ctrlKey;
  if (cmd && !e.altKey && !e.shiftKey) {
    if (e.key.toLowerCase() === "c") return "copy";
    if (e.key.toLowerCase() === "x") return "cut";
    if (e.key.toLowerCase() === "v") return "paste";
  }
  if (!cmd && e.altKey && !e.shiftKey) {
    if (e.key === "[") return "trim-start";
    if (e.key === "]") return "trim-end";
  }
}
export function trimAtPlayhead(
  p: Project,
  id: string,
  time: number,
  side: "trim-start" | "trim-end",
): Project {
  const c = p.clips.find((c) => c.id === id);
  if (!c || !Number.isFinite(time)) return p;
  const at = Math.round(time * p.fps) / p.fps;
  const start = c.start ?? 0;
  if (
    at < start + 1 / p.fps - 1e-8 ||
    at > start + duration(c) - 1 / p.fps + 1e-8
  )
    return p;
  const source = c.trimIn + (at - start) * c.speed;
  return {
    ...p,
    clips: p.clips.map((clip) => {
      if (clip.id === id)
        return side === "trim-start"
          ? { ...clip, start: at, trimIn: source, transition: undefined }
          : { ...clip, trimOut: source };
      if (side === "trim-end" && clip.transition?.fromClipId === id)
        return { ...clip, transition: undefined };
      return clip;
    }),
  };
}
export function pasteClip(
  p: Project,
  saved: Clip,
  time: number,
): { project: Project; id?: string } {
  const asset = p.assets.find((a) => a.id === saved.assetId);
  if (!asset || !Number.isFinite(time)) return { project: p };
  const track = p.tracks.find((t) => t.id === saved.trackId);
  if (!track) return { project: p };
  const clip = {
    ...structuredClone(saved),
    id: uid(),
    start: Math.max(0, Math.round(time * p.fps) / p.fps),
    trackId: track.id,
    transition: undefined,
  };
  return { project: { ...p, clips: [...p.clips, clip] }, id: clip.id };
}
export function requestClipEdit(command: ClipEdit, id: string) {
  window.dispatchEvent(
    new CustomEvent("studio-clip-edit", { detail: { command, id } }),
  );
}
