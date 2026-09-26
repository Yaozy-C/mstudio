import { uid, type Project } from "../model";
import { addTrack } from "./document";

export function detachAudio(project: Project, clipId: string): Project {
  const clip = project.clips.find((c) => c.id === clipId);
  const asset = project.assets.find((a) => a.id === clip?.assetId);
  const track = project.tracks.find((t) => t.id === clip?.trackId);
  if (
    !clip ||
    asset?.kind !== "video" ||
    !asset.hasAudio ||
    track?.kind !== "video"
  )
    return project;
  const next = addTrack(project, "audio");
  return {
    ...next,
    tracks: next.tracks.map((t) =>
      t.id === next.tracks.at(-1)!.id ? { ...t, muted: track.muted } : t,
    ),
    clips: [
      ...next.clips.map((c) => (c.id === clipId ? { ...c, volume: 0 } : c)),
      { ...clip, id: uid(), trackId: next.tracks.at(-1)!.id },
    ],
  };
}
