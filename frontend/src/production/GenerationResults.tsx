import { t } from "../i18n";
import type { Project } from "../model";
import type { ProductionTask } from "./types";
import type { ProductionController } from "./useProduction";
import { GenerationResult } from "./GenerationResult";

export function GenerationResults({
  task,
  project,
  canvas,
}: {
  task: ProductionTask;
  project: Project;
  canvas: ProductionController;
}) {
  const ids =
    task.resultAssetIds ?? (task.resultAssetId ? [task.resultAssetId] : []);
  if (!ids.length) return null;
  return (
    <section className="run-results">
      {task.kind === "image" && (
        <p role="status" className="run-result-count">
          {t("已生成")} {ids.length} {t("张图片")}
        </p>
      )}
      <div
        className={
          task.kind === "image"
            ? "generation-results-grid"
            : "generation-results-video"
        }
      >
        {ids.map((id) => (
          <GenerationResult
            key={id}
            task={{ ...task, resultAssetId: id }}
            project={project}
            reference={(assetId) => {
              canvas.setTaskPanelOpen(false);
              canvas.attach({ kind: "asset", id: assetId });
            }}
            locate={() => {
              canvas.setTaskPanelOpen(false);
              window.dispatchEvent(new Event("studio-show-canvas"));
              const item = canvas.items.find((n) => n.assetId === id);
              if (item) canvas.focusItem(item.key);
            }}
          />
        ))}
      </div>
    </section>
  );
}
