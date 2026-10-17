import { TrackIdentity } from "./TrackIdentity";
import { IconButton } from "../ui/IconButton";
import { Trash } from "@phosphor-icons/react";
import { t } from "../i18n";
import type { Project } from "../model";
import {
  captionTracksOf,
  removeCaptionTrack,
  type CaptionLane,
} from "./captionTracks";
function CaptionTrackHeading({
  track,
  change,
}: {
  track: CaptionLane;
  change: (f: (p: Project) => Project) => void;
}) {
  return (
    <div className="track-heading caption-heading">
      <TrackIdentity
        name={track.name}
        kind="caption"
        rename={(name) =>
          change((p) => ({
            ...p,
            captionTracks: captionTracksOf(p).map((v) =>
              v.id === track.id ? { ...v, name } : v,
            ),
          }))
        }
      />
      <IconButton
        icon={Trash}
        label={t("删除轨道 {v0}", { v0: track.name })}
        onClick={() => change((p) => removeCaptionTrack(p, track.id))}
      />
    </div>
  );
}

export function CaptionTrackHeadings({
  project,
  change,
}: {
  project: Project;
  change: (f: (p: Project) => Project) => void;
}) {
  return captionTracksOf(project).map((track) => (
    <CaptionTrackHeading key={track.id} track={track} change={change} />
  ));
}
