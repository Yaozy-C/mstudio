import { uid, type Project, type Caption } from "../model";
import { t } from "../i18n";
export type CaptionLane = { id: string; name: string };
type Document = Pick<Project, "captionTracks" | "captions">;
export function captionTracksOf(p: Document): CaptionLane[] {
  return p.captionTracks?.length
    ? p.captionTracks
    : [{ id: "captions", name: t("字幕") }];
}
export function captionTrackId(p: Document, c: Caption) {
  const tracks = captionTracksOf(p);
  return tracks.some((t) => t.id === c.trackId) ? c.trackId! : tracks[0].id;
}
export function addCaptionTrack(p: Project): Project {
  const tracks = captionTracksOf(p);
  return {
    ...p,
    captionTracks: [
      ...tracks,
      { id: uid(), name: `${t("字幕")} ${tracks.length + 1}` },
    ],
  };
}
export function removeCaptionTrack(p: Project, id: string): Project {
  const tracks = captionTracksOf(p);
  if (!tracks.some((t) => t.id === id)) return p;
  return {
    ...p,
    captionTracks: tracks.filter((t) => t.id !== id),
    captions: p.captions.filter((c) => captionTrackId(p, c) !== id),
  };
}
/** Last caption is composited on top, matching the top timeline lane. */
export function orderedCaptions(p: Document): Caption[] {
  const tracks = captionTracksOf(p);
  return [...p.captions].sort(
    (a, b) =>
      tracks.findIndex((t) => t.id === captionTrackId(p, b)) -
      tracks.findIndex((t) => t.id === captionTrackId(p, a)),
  );
}
