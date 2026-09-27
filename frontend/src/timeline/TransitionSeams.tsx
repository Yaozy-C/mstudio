import { t, useLanguage } from "../i18n";
import { useState } from "react";
import { Popover } from "@radix-ui/themes";
import { duration, type Clip, type Project } from "../model";
import { requestCreativeTask } from "../creative/aiTasks";
import {
  seams,
  setTransition,
  transitionKinds,
  type Transition,
} from "./transitions";
import type { PlaybackClock } from "./clock";
import "../styles/transitions.css";
export function TransitionSeams({
  project,
  trackId,
  zoom,
  clock,
  change,
}: {
  project: Project;
  trackId: string;
  zoom: number;
  clock: PlaybackClock;
  change: (f: (p: Project) => Project) => void;
}) {
  useLanguage();
  if (trackId !== project.tracks.find((t) => t.kind === "video")?.id)
    return null;
  return seams(project, trackId).map(({ left, right }) => (
    <Seam
      key={`${left.id}:${right.id}`}
      {...{ project, left, right, zoom, clock, change }}
    />
  ));
}
function Seam({
  project,
  left,
  right,
  zoom,
  clock,
  change,
}: {
  project: Project;
  left: Clip;
  right: Clip;
  zoom: number;
  clock: PlaybackClock;
  change: (f: (p: Project) => Project) => void;
}) {
  useLanguage();
  const [open, setOpen] = useState(false),
    [error, setError] = useState("");
  const value =
    right.transition?.fromClipId === left.id ? right.transition : undefined;
  const max = Math.min(3, duration(left), duration(right));
  function apply(
    kind: Transition["kind"] | null,
    seconds = value?.duration ?? Math.min(0.5, max),
  ) {
    try {
      const next = setTransition(project, left.id, right.id, kind, seconds);
      clock.pause();
      change(() => next);
      setError("");
    } catch (e) {
      setError(String(e));
    }
  }
  return (
    <Popover.Root open={open} onOpenChange={setOpen}>
      <Popover.Trigger>
        <button
          className={`transition-seam ${value ? "active" : ""}`}
          style={{ left: (right.start ?? 0) * zoom }}
          aria-label={
            value
              ? t("编辑转场 {v0}", { v0: t(transitionKinds[value.kind]) })
              : t("添加转场")
          }
          title={
            value
              ? t("{v0} · {v1} 秒", {
                  v0: t(transitionKinds[value.kind]),
                  v1: value.duration,
                })
              : t("在接缝添加转场")
          }
        >
          ◇
        </button>
      </Popover.Trigger>
      <Popover.Content className="transition-editor" width="300px">
        <h3>{t("片段转场")}</h3>
        <label>
          {t("效果")}
          <select
            value={value?.kind ?? "none"}
            onChange={(e) =>
              apply(
                e.target.value === "none"
                  ? null
                  : (e.target.value as Transition["kind"]),
              )
            }
          >
            <option value="none">{t("直接切换")}</option>
            {Object.entries(transitionKinds).map(([key, title]) => (
              <option key={key} value={key}>
                {t(title)}
              </option>
            ))}
          </select>
        </label>
        <label>
          {t("时长 ·")} {value?.duration ?? Math.min(0.5, max)} {t("秒")}
          <input
            aria-label={t("转场时长")}
            type="range"
            min={0.05}
            max={max}
            step={0.05}
            value={value?.duration ?? Math.min(0.5, max)}
            disabled={!value}
            onChange={(e) => apply(value!.kind, +e.target.value)}
          />
        </label>
        <p>
          {t(
            "保持剪辑、声音和字幕时间。优先使用切点外的素材余量，不足时延展边缘帧。",
          )}
        </p>
        <button
          onClick={() => {
            clock.pause();
            clock.seek(
              Math.max(0, (right.start ?? 0) - (value?.duration ?? 0.5)),
            );
            setOpen(false);
          }}
        >
          {t("定位预览")}
        </button>
        <button
          onClick={() => {
            setOpen(false);
            requestCreativeTask({
              agentId: "transition-designer",
              refs: [
                { kind: "clip", id: left.id },
                { kind: "clip", id: right.id },
              ],
              text: t(
                "请为这两个相邻片段设计并添加合适的转场，优先保持动作、视线和节奏连续，说明选择理由。",
              ),
            });
          }}
        >
          {t("让转场 Agent 设计")}
        </button>
        {value && <button onClick={() => apply(null)}>{t("移除转场")}</button>}
        {error && <p role="alert">{t(error)}</p>}
      </Popover.Content>
    </Popover.Root>
  );
}
