import { Trash, Subtitles } from "@phosphor-icons/react";
import { t } from "../i18n";
import type { Project } from "../model";
import {
  captionTracksOf,
  removeCaptionTrack,
  type CaptionLane,
} from "./captionTracks";
export function CaptionTrackHeading({
  track,
  change,
}: {
  track: CaptionLane;
  change: (f: (p: Project) => Project) => void;
}) {
  return (
    <div className="track-heading caption-heading">
      <Subtitles size={16} aria-hidden="true" />
      <div className="track-identity">
        <input
          aria-label={t("{v0} 轨道名称", { v0: track.name })}
          value={track.name}
          onChange={(e) => {
            const name = e.target.value;
            change((p) => ({
              ...p,
              captionTracks: captionTracksOf(p).map((v) =>
                v.id === track.id ? { ...v, name } : v,
              ),
            }));
          }}
        />
      </div>
      <button
        title={t("删除轨道 {v0}", { v0: track.name })}
        aria-label={t("删除轨道 {v0}", { v0: track.name })}
        onClick={() => change((p) => removeCaptionTrack(p, track.id))}
      >
        <Trash size={14} />
      </button>
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
