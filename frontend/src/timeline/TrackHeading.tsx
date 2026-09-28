import { t, useLanguage } from "../i18n";
import {
  ArrowsHorizontal,
  Eye,
  EyeSlash,
  SpeakerHigh,
  SpeakerSlash,
  Trash,
} from "@phosphor-icons/react";
import { ObjectMenu } from "../ui/ObjectMenu";
import type { Project, Track } from "../model";
import { packClips } from "./packClips";
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
  useLanguage();
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
    ? t("删除轨道及其中 {v0} 个片段（可撤销）", { v0: count })
    : t("删除空轨道（可撤销）");
  return (
    <ObjectMenu actions={[{ label: title, run: remove, danger: true }]}>
      <div className="track-heading">
        <div className="track-identity">
          <input
            aria-label={t("{v0} 轨道名称", { v0: track.name })}
            title={track.name}
            value={track.name}
            onChange={(e) => patch({ name: e.target.value })}
          />
          {(track.muted || track.hidden) && (
            <small className="track-state">
              {[track.muted && t("已静音"), track.hidden && t("画面已隐藏")]
                .filter(Boolean)
                .join(" · ")}
            </small>
          )}
        </div>
        <button
          title={track.muted ? t("取消静音") : t("静音此轨道")}
          aria-label={`${track.name}：${track.muted ? t("取消静音") : t("静音")}`}
          aria-pressed={!!track.muted}
          onClick={() => patch({ muted: !track.muted })}
        >
          {track.muted ? <SpeakerSlash size={14} /> : <SpeakerHigh size={14} />}
        </button>
        {track.kind === "video" && (
          <button
            title={
              track.hidden ? t("显示画面") : t("隐藏画面，声音保持当前设置")
            }
            aria-label={`${track.name}：${track.hidden ? t("显示画面") : t("隐藏画面")}`}
            aria-pressed={!!track.hidden}
            onClick={() => patch({ hidden: !track.hidden })}
          >
            {track.hidden ? <EyeSlash size={14} /> : <Eye size={14} />}
          </button>
        )}
        <button
          title={t("此轨道按当前顺序从 0 秒首尾相接（可撤销）")}
          aria-label={t("排片轨道 {v0}", { v0: track.name })}
          disabled={!count}
          onClick={() => onChange((p) => packClips(p, track.id))}
        >
          <ArrowsHorizontal size={14} />
        </button>
        <button
          className="delete-track"
          title={title}
          aria-label={t("删除轨道 {v0}", { v0: track.name })}
          onClick={remove}
        >
          <Trash size={14} />
        </button>
      </div>
    </ObjectMenu>
  );
}
