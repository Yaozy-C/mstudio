import type { Track } from "../model";
export function trackRows(tracks: Track[]): Track[] {
  const video = tracks.filter((t) => t.kind === "video").reverse();
  const audio = tracks.filter((t) => t.kind === "audio");
  return [
    ...video.flatMap((t) => [
      t,
      ...audio.filter((a) => a.sourceTrackId === t.id),
    ]),
    ...audio.filter((a) => !video.some((t) => t.id === a.sourceTrackId)),
  ];
}
