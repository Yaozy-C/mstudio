import {
  Eye,
  EyeSlash,
  SpeakerHigh,
  SpeakerSlash,
  Trash,
} from "@phosphor-icons/react";
import { DropdownMenu } from "@radix-ui/themes";
import { removeTrack } from "./document";
import type { Project, Track } from "../model";
export function InspectorTrackActions({
  track,
  project,
  change,
}: {
  track: Track;
  project: Project;
  change: (fn: (p: Project) => Project) => void;
}) {
  const patch = (fields: Partial<Track>) =>
    change((p) => ({
      ...p,
      tracks: p.tracks.map((t) =>
        t.id === track.id ? { ...t, ...fields } : t,
      ),
    }));
  const count = project.clips.filter((c) => c.trackId === track.id).length;
  return (
    <footer className="inspector-track-actions">
      <span title={track.name}>轨道操作</span>
      <button
        aria-label={track.muted ? "取消轨道静音" : "静音轨道"}
        title={track.muted ? "取消轨道静音" : "静音轨道"}
        aria-pressed={!!track.muted}
        onClick={() => patch({ muted: !track.muted })}
      >
        {track.muted ? <SpeakerSlash size={21} /> : <SpeakerHigh size={21} />}
      </button>
      {track.kind === "video" && (
        <button
          aria-label={track.hidden ? "显示轨道画面" : "隐藏轨道画面"}
          title={track.hidden ? "显示轨道画面" : "隐藏轨道画面"}
          aria-pressed={!!track.hidden}
          onClick={() => patch({ hidden: !track.hidden })}
        >
          {track.hidden ? <EyeSlash size={21} /> : <Eye size={21} />}
        </button>
      )}
      <DropdownMenu.Root>
        <DropdownMenu.Trigger>
          <button title="删除轨道" aria-label="删除轨道">
            <Trash size={21} />
          </button>
        </DropdownMenu.Trigger>
        <DropdownMenu.Content className="studio-menu">
          <DropdownMenu.Item
            color="red"
            onSelect={() => change((p) => removeTrack(p, track.id))}
          >
            删除轨道及 {count} 个片段（可撤销）
          </DropdownMenu.Item>
        </DropdownMenu.Content>
      </DropdownMenu.Root>
    </footer>
  );
}
