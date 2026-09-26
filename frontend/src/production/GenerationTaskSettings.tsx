import type { Project } from "../model";
import type { ProductionController } from "./useProduction";
import { GenerationSettings } from "./GenerationSettings";

export function GenerationTaskSettings({
  project,
  canvas,
}: {
  project: Project;
  canvas: ProductionController;
}) {
  if (!canvas.configuration) return null;
  return (
    <GenerationSettings
      key={canvas.configuration.key}
      task={canvas.configuration}
      project={project}
      models={canvas.media.models}
      close={() => canvas.configure(null)}
      save={(configured, start) => {
        const task = configured;
        canvas.update(task, task.key);
        canvas.showTask(task);
        if (!task.turnId) canvas.preferences({ [task.kind]: task.modelId });
        if (start) void canvas.submit(task);
      }}
    />
  );
}
