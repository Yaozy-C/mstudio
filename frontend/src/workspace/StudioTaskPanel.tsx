import { TaskPanel } from "../production/TaskPanel";
import type { Project } from "../model";
import type { ProductionController } from "../production/useProduction";
export function StudioTaskPanel({
  project,
  canvas,
  onModels,
}: {
  project: Project;
  canvas: ProductionController;
  onModels: () => void;
}) {
  return (
    <TaskPanel
      project={project}
      canvas={canvas}
      settings={() => {
        canvas.setTaskPanelOpen(false);
        onModels();
      }}
    />
  );
}
