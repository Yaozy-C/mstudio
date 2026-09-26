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
  const audioTrack = {
    ...next.tracks.at(-1)!,
    muted: track.muted,
    sourceTrackId: track.id,
  };
  const position = project.tracks.findIndex((t) => t.id === track.id) + 1;
  return {
    ...next,
    tracks: [
      ...project.tracks.slice(0, position),
      audioTrack,
      ...project.tracks.slice(position),
    ],
    clips: [
      ...next.clips.map((c) => (c.id === clipId ? { ...c, volume: 0 } : c)),
      { ...clip, id: uid(), trackId: audioTrack.id },
    ],
  };
}
