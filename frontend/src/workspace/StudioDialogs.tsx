import { useEffect } from "react";
import type { PlaybackClock } from "../timeline/clock";
import { Settings } from "./Settings";
import { ExportDialog } from "./Export";
import type { Project } from "../model";
export type StudioDialogKind =
  "settings" | "models" | "agents" | "export" | null;
export function StudioDialogs({
  kind,
  project,
  close,
  clock,
}: {
  kind: StudioDialogKind;
  clock: PlaybackClock;
  project: Project;
  close: () => void;
}) {
  const settingsOpen = ["settings", "models", "agents"].includes(kind ?? "");
  useEffect(() => {
    if (settingsOpen) clock.pause();
  }, [settingsOpen, clock]);
  if (kind === "agents")
    return <Settings project={project} initialTab="agents" onClose={close} />;
  if (kind === "settings")
    return <Settings project={project} onClose={close} />;
  if (kind === "models")
    return <Settings project={project} initialTab="models" onClose={close} />;
  if (kind === "export")
    return <ExportDialog project={project} onClose={close} />;
  return null;
}
