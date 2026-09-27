import {
  Eye,
  EyeSlash,
  SpeakerHigh,
  SpeakerSlash,
  Trash,
} from "@phosphor-icons/react";
import { ObjectMenu } from "../ui/ObjectMenu";
import type { Project, Track } from "../model";
import { removeTrack } from "./document";
export function TrackHeading({
  track,
  project,
  onChange,
  onRemove,
}: {
  track: Track;
  project: Project;
  onChange: (fn: (p: Project) => Project) => void;
  onRemove: () => void;
}) {
  const count = project.clips.filter((c) => c.trackId === track.id).length;
  const remove = () => {
    onRemove();
    onChange((p) => removeTrack(p, track.id));
  };
  const patch = (changes: Partial<Track>) =>
    onChange((p) => ({
      ...p,
      tracks: p.tracks.map((t) =>
        t.id === track.id ? { ...t, ...changes } : t,
      ),
    }));
  const title = count
    ? `删除轨道及其中 ${count} 个片段（可撤销）`
    : "删除空轨道（可撤销）";
  return (
    <ObjectMenu actions={[{ label: title, run: remove, danger: true }]}>
      <div className="track-heading">
        <div className="track-identity">
          <input
            aria-label={`${track.name} 轨道名称`}
            title={track.name}
            value={track.name}
            onChange={(e) => patch({ name: e.target.value })}
          />
          {(track.muted || track.hidden) && (
            <small className="track-state">
              {[track.muted && "已静音", track.hidden && "画面已隐藏"]
                .filter(Boolean)
                .join(" · ")}
            </small>
          )}
        </div>
        <button
          title={track.muted ? "取消静音" : "静音此轨道"}
          aria-label={`${track.name}：${track.muted ? "取消静音" : "静音"}`}
          aria-pressed={!!track.muted}
          onClick={() => patch({ muted: !track.muted })}
        >
          {track.muted ? <SpeakerSlash size={14} /> : <SpeakerHigh size={14} />}
        </button>
        {track.kind === "video" && (
          <button
            title={track.hidden ? "显示画面" : "隐藏画面，声音保持当前设置"}
            aria-label={`${track.name}：${track.hidden ? "显示画面" : "隐藏画面"}`}
            aria-pressed={!!track.hidden}
            onClick={() => patch({ hidden: !track.hidden })}
          >
            {track.hidden ? <EyeSlash size={14} /> : <Eye size={14} />}
          </button>
        )}
        <button
          className="delete-track"
          title={title}
          aria-label={`删除轨道 ${track.name}`}
          onClick={remove}
        >
          <Trash size={14} />
        </button>
      </div>
    </ObjectMenu>
  );
}
