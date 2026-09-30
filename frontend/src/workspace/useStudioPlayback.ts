export { useCreationTab } from "../timeline/transitionDock";
import { useEffect, useState } from "react";
import { PlaybackClock } from "../timeline/clock";
import { appendAsset, endTime } from "../timeline/document";
import { splitSelected } from "../timeline/splitSelected";
import type { Asset, Project } from "../model";
import type { ChangeProject } from "../production/types";
export function useStudioPlayback(
  project: Project,
  change: ChangeProject,
  show: () => void,
  clearSelection: () => void,
) {
  const [clock] = useState(() => new PlaybackClock());
  useEffect(() => {
    clock.configure(endTime(project), project.fps);
  }, [clock, project.clips, project.captions, project.fps]);
  useEffect(() => () => clock.dispose(), [clock]);
  return {
    clock,
    split: (clipId: string | null) => {
      clock.pause();
      change((p) => splitSelected(p, clipId, clock.getSnapshot().time));
    },
    play: () => {
      show();
      clock.toggle();
    },
    add: (asset: Asset) => {
      clock.pause();
      change((p) => appendAsset(p, asset));
      clearSelection();
    },
  };
}
