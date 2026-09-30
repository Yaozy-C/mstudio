import { InspectorControl } from "./InspectorControl";
import { useState } from "react";
import { t, useLanguage } from "../i18n";
import { duration, type Project, type Clip } from "../model";
import { DockPanel } from "../ui/DockPanel";
import {
  seams,
  setTransition,
  transitionKinds,
  type Transition,
} from "./transitions";
import { defaultDesign, type TransitionDesign } from "./transitionDesign";
import { TransitionDesignFields } from "./TransitionDesignFields";
import { TransitionCard } from "./TransitionCard";
import type { PlaybackClock } from "./clock";
import "../styles/transitions.css";
type Props = {
  project: Project;
  clock: PlaybackClock;
  change: (f: (p: Project) => Project) => void;
  onClose: () => void;
  leftId: string;
  rightId: string;
};
export function TransitionPanel(props: Props) {
  useLanguage();
  const pair = seams(
    props.project,
    props.project.tracks.find((t) => t.kind === "video")?.id ?? "",
  ).find(
    ({ left, right }) => left.id === props.leftId && right.id === props.rightId,
  );
  return (
    <DockPanel id="creation" title={t("片段转场")} onClose={props.onClose}>
      {pair ? (
        <TransitionEditor {...props} {...pair} />
      ) : (
        <p>{t("请选择两个相邻片段之间的转场。")}</p>
      )}
    </DockPanel>
  );
}
function TransitionEditor({
  project,
  clock,
  change,
  left,
  right,
}: Props & { left: Clip; right: Clip }) {
  const [error, setError] = useState("");
  const value =
    right.transition?.fromClipId === left.id ? right.transition : undefined;
  const max = Math.min(3, duration(left), duration(right));
  function apply(
    kind: Transition["kind"] | null,
    seconds = value?.duration ?? Math.min(0.5, max),
    design?: TransitionDesign,
  ) {
    try {
      const next = setTransition(
        project,
        left.id,
        right.id,
        kind,
        seconds,
        kind === "custom"
          ? (design ?? value?.design ?? defaultDesign())
          : undefined,
      );
      clock.pause();
      change(() => next);
      setError("");
    } catch (e) {
      setError(String(e));
    }
  }
  return (
    <div className="transition-editor">
      <p>{t("点击卡片应用转场，悬停查看动态效果。")}</p>
      <div className="transition-cards">
        {[["none", "直接切换"], ...Object.entries(transitionKinds)].map(
          ([kind, title]) => (
            <TransitionCard
              key={kind}
              kind={kind}
              title={t(title)}
              selected={(value?.kind ?? "none") === kind}
              onClick={() =>
                apply(kind === "none" ? null : (kind as Transition["kind"]))
              }
            />
          ),
        )}
      </div>
      {value && (
        <InspectorControl
          label={t("转场时长")}
          value={value.duration}
          unit={t("秒")}
          min={0.05}
          max={max}
          step={0.05}
          slider
          change={(seconds) => apply(value.kind, seconds)}
        />
      )}
      {value?.kind === "circleopen" && (
        <p>{t("后一段画面从圆心向外扩大，直到铺满画面。")}</p>
      )}
      {value?.kind === "circleclose" && (
        <p>{t("前一段画面在圆内缩小，露出后一段画面。")}</p>
      )}
      {value?.kind === "custom" && (
        <>
          <p>{t("自定义卡片展示组合示例，调整参数后请在时间线预览。")}</p>
          <TransitionDesignFields
            design={value.design!}
            change={(design) => apply("custom", value.duration, design)}
          />
        </>
      )}
      {error && <p role="alert">{t(error)}</p>}
    </div>
  );
}
