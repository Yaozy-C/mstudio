import { t, useLanguage } from "../i18n";
import {
  ArrowsOutSimple,
  Scissors,
  Trash,
  Minus,
  Plus,
  CaretDown,
} from "@phosphor-icons/react";
import type { Project } from "../model";
import type { PlaybackClock } from "./clock";
import { TimelineTransport } from "./TimelineTransport";
import { ClockReadout } from "./ClockReadout";
import { frameTime } from "./geometry";
import { ShortcutHelp } from "./ShortcutHelp";
import { packClips } from "./packClips";
import { addTrack } from "./document";
export function TimelineToolbar({
  selectedCaption,
  onRemoveCaption,
  onPlay,
  onFit,
  project,
  clock,
  selected,
  onSelect,
  onChange,
  onSplit,
  onCollapse,
  changeZoom,
  zoom,
  total,
}: {
  selectedCaption?: string | null;
  onRemoveCaption?: () => void;
  onPlay: () => void;
  onFit: () => void;
  project: Project;
  clock: PlaybackClock;
  selected: string | null;
  onSelect: (id: string | null) => void;
  onChange: (fn: (p: Project) => Project) => void;
  onSplit: () => void;
  onCollapse: () => void;
  changeZoom: (factor: number) => void;
  zoom: number;
  total: number;
}) {
  useLanguage();
  return (
    <header className="timeline-header">
      <h3>
        {t("时间线")}{" "}
        <span>
          {project.clips.length} {t("个片段")}
        </span>
      </h3>
      <TimelineTransport clock={clock} onPlay={onPlay} />
      <div className="timeline-edit">
        <button
          title={t("分割 ⌘B")}
          disabled={!!selectedCaption}
          onClick={onSplit}
        >
          <Scissors />
          {t("分割")}
        </button>
        <button
          title={selectedCaption ? t("删除字幕 Delete") : t("删除片段 Delete")}
          disabled={!selected && !selectedCaption}
          onClick={() => {
            if (selectedCaption) {
              onRemoveCaption?.();
              return;
            }
            onChange((p) => ({
              ...p,
              clips: p.clips.filter((c) => c.id !== selected),
            }));
            onSelect(null);
          }}
        >
          <Trash />
        </button>
        <button onClick={() => onChange((p) => addTrack(p, "video"))}>
          {t("＋画面轨")}
        </button>
        <button onClick={() => onChange((p) => addTrack(p, "audio"))}>
          {t("＋音轨")}
        </button>
        <button
          title={t("所有轨道按当前顺序从 0 秒首尾相接（可撤销）")}
          disabled={!project.clips.length}
          onClick={() => {
            clock.pause();
            onChange((p) => packClips(p));
          }}
        >
          {t("一键排片")}
        </button>
      </div>
      <span className="time-display">
        <ClockReadout clock={clock} />
        <i> / {frameTime(total, project.fps)}</i>
      </span>
      <div className="zoom">
        <ShortcutHelp />
        <button aria-label={t("适合整条时间线")} onClick={onFit}>
          <ArrowsOutSimple />
        </button>
        <button
          aria-label={t("缩小时间精度")}
          title="⌘−"
          onClick={() => changeZoom(1 / 1.5)}
        >
          <Minus />
        </button>
        <span>{Math.round((zoom / 64) * 100)}%</span>
        <button
          aria-label={t("放大时间精度")}
          title="⌘+"
          onClick={() => changeZoom(1.5)}
        >
          <Plus />
        </button>
      </div>
      <button
        className="icon-button"
        onClick={onCollapse}
        title={t("收起时间线 T")}
      >
        <CaretDown />
      </button>
    </header>
  );
}
