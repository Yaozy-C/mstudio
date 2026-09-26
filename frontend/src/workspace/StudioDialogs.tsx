import { Settings } from "./Settings";
import { ExportDialog } from "./Export";
import type { Project } from "../model";
export function StudioDialogs({
  kind,
  project,
  close,
}: {
  kind: "settings" | "models" | "agents" | "export" | null;
  project: Project;
  close: () => void;
}) {
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
