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
import { ObjectActions } from "../ui/ObjectActions";
import { IconButton } from "../ui/IconButton";
import { TrackIdentity } from "./TrackIdentity";
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
      tracks: p.tracks.map((item) =>
        item.id === track.id ? { ...item, ...changes } : item,
      ),
    }));
  const actions = [
    {
      icon: ArrowsHorizontal,
      label: t("排片轨道 {v0}", { v0: track.name }),
      disabled: !count,
      run: () => onChange((p) => packClips(p, track.id)),
    },
    {
      icon: Trash,
      label: count
        ? t("删除轨道及其中 {v0} 个片段（可撤销）", { v0: count })
        : t("删除空轨道（可撤销）"),
      run: remove,
      danger: true,
    },
  ];
  return (
    <ObjectMenu actions={actions}>
      <div className="track-heading" role="group" aria-label={track.name}>
        <TrackIdentity
          name={track.name}
          kind={track.kind}
          rename={(name) => patch({ name })}
        />
        <IconButton
          icon={track.muted ? SpeakerSlash : SpeakerHigh}
          label={`${track.name}：${track.muted ? t("取消静音") : t("静音")}`}
          aria-pressed={!!track.muted}
          onClick={() => patch({ muted: !track.muted })}
        />
        {track.kind === "video" && (
          <IconButton
            icon={track.hidden ? EyeSlash : Eye}
            label={`${track.name}：${track.hidden ? t("显示画面") : t("隐藏画面")}`}
            aria-pressed={!!track.hidden}
            onClick={() => patch({ hidden: !track.hidden })}
          />
        )}
        <ObjectActions actions={actions} />
      </div>
    </ObjectMenu>
  );
}
