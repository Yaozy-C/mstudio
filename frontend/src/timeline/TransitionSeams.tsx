import { t, useLanguage } from "../i18n";
import type { Project } from "../model";
import { seams, transitionKinds } from "./transitions";
import type { PlaybackClock } from "./clock";
import { openTransitionDock } from "./transitionDock";
import "../styles/transitions.css";
export function TransitionSeams({
  project,
  trackId,
  zoom,
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
  return seams(project, trackId).map(({ left, right }) => {
    const value =
      right.transition?.fromClipId === left.id ? right.transition : undefined;
    return (
      <button
        key={`${left.id}:${right.id}`}
        className={`transition-seam ${value ? "active" : ""}`}
        style={{ left: (right.start ?? 0) * zoom }}
        aria-label={
          value
            ? t("编辑转场 {v0}", { v0: t(transitionKinds[value.kind]) })
            : t("添加转场")
        }
        title={t("在接缝添加转场")}
        onClick={() => openTransitionDock(left.id, right.id)}
      >
        ◇
      </button>
    );
  });
}
