import type { ComponentProps } from "react";
import { CaptionTrack } from "./CaptionTrack";
import { captionTracksOf } from "./captionTracks";
export function CaptionTrackRows(
  props: Omit<ComponentProps<typeof CaptionTrack>, "trackId">,
) {
  return captionTracksOf(props.project).map((track) => (
    <CaptionTrack key={track.id} trackId={track.id} {...props} />
  ));
}
