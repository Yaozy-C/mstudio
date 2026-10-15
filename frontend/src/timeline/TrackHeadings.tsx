import { Stack } from "@phosphor-icons/react";
import { t } from "../i18n";
import type { Project, Track } from "../model";
import type { PlaybackClock } from "./clock";
import { TrackHeading } from "./TrackHeading";
import { CaptionTrackHeadings } from "./CaptionTrackHeading";

export function TrackHeadings({
  rows,
  project,
  onChange,
  clock,
  selected,
  onSelect,
}: {
  rows: Track[];
  project: Project;
  onChange: (fn: (project: Project) => Project) => void;
  clock: PlaybackClock;
  selected: string | null;
  onSelect: (id: string | null) => void;
}) {
  return (
    <div className="track-labels">
      <div
        title={t("轨道 · 上层覆盖下层")}
        aria-label={t("轨道 · 上层覆盖下层")}
      >
        <Stack size={16} aria-hidden="true" />
      </div>
      {rows.map((track) => (
        <TrackHeading
          key={track.id}
          track={track}
          project={project}
          onChange={onChange}
          onRemove={() => {
            clock.pause();
            if (
              project.clips.some(
                (clip) => clip.id === selected && clip.trackId === track.id,
              )
            )
              onSelect(null);
          }}
        />
      ))}
      <CaptionTrackHeadings project={project} change={onChange} />
    </div>
  );
}
