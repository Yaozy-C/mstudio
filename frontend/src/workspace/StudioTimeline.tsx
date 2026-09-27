import { t, useLanguage } from "../i18n";
import { Timeline } from "../timeline/Timeline";
import type { PlaybackClock } from "../timeline/clock";
import type { Project } from "../model";
import type { ChangeProject } from "../production/types";
export function StudioTimeline({
  project,
  clock,
  visible,
  selected,
  change,
  split,
  play,
  toggle,
  select,
  reference,
  captions,
}: {
  captions: () => void;
  project: Project;
  clock: PlaybackClock;
  visible: boolean;
  selected: string | null;
  change: ChangeProject;
  split: () => void;
  play: () => void;
  toggle: () => void;
  select: (id: string | null, inspect: boolean) => void;
  reference: (id: string) => void;
}) {
  useLanguage();
  return visible ? (
    <Timeline
      project={project}
      clock={clock}
      selected={selected}
      onChange={change}
      onPlay={play}
      onSplit={split}
      onCollapse={toggle}
      onOpen={(id) => select(id, true)}
      onSelect={(id) => select(id, false)}
      onReference={reference}
      onCaption={captions}
    />
  ) : (
    <button className="timeline-collapsed" onClick={toggle}>
      {t("展开时间线 · T")}
    </button>
  );
}
