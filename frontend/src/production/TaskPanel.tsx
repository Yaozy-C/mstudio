import { Dialog } from "@radix-ui/themes";
import { t, useLanguage } from "../i18n";
import { ListChecks, X, Image, VideoCamera } from "@phosphor-icons/react";
import type { Project } from "../model";
import type { ProductionController } from "./useProduction";
import { GenerationRun } from "./GenerationRun";
import { pendingRun, runStatuses } from "./useRunActions";
import "./task-panel.css";

export function TaskPanel({
  project,
  canvas,
  settings,
}: {
  project: Project;
  canvas: ProductionController;
  settings: () => void;
}) {
  useLanguage();
  const runs = [...canvas.runs].reverse();
  const visible = runs.filter(
    (t) => !t.hiddenFromList || t.key === canvas.activeTaskKey,
  );
  const selected =
    visible.find((t) => t.key === canvas.activeTaskKey) ?? visible[0];
  const active = visible.filter(pendingRun).length;
  const failed = visible.filter((t) => t.status === "FAILED").length;
  return (
    <Dialog.Root
      open={canvas.taskPanelOpen}
      onOpenChange={canvas.setTaskPanelOpen}
    >
      <Dialog.Trigger>
        <button type="button" className="chrome-action task-dialog-trigger">
          <ListChecks />
          {t("生成任务")} <span>{visible.length}</span>
          {!!active && (
            <small className="task-active">
              {active} {t("个进行中")}
            </small>
          )}
          {!!failed && (
            <small className="task-failed">
              {failed} {t("个失败")}
            </small>
          )}
        </button>
      </Dialog.Trigger>
      <Dialog.Content
        className="generation-tasks-dialog"
        aria-describedby={undefined}
      >
        <header className="task-panel-heading">
          <Dialog.Title>{t("生成任务")}</Dialog.Title>
          <Dialog.Close>
            <button type="button" aria-label={t("关闭生成任务")}>
              <X />
            </button>
          </Dialog.Close>
        </header>
        <div className="task-panel-body">
          <div className="task-panel-list" aria-label={t("任务列表")}>
            {!visible.length && (
              <p>
                {t(
                  runs.length
                    ? "列表已清空，任务记录和素材仍保留。"
                    : "暂无生成任务；创建后会显示在这里。",
                )}
              </p>
            )}
            {visible.map((task) => {
              const Icon = task.kind === "image" ? Image : VideoCamera;
              return (
                <button
                  type="button"
                  key={task.key}
                  aria-pressed={selected?.key === task.key}
                  className={selected?.key === task.key ? "selected" : ""}
                  onClick={() => canvas.showTask(task)}
                >
                  <Icon size={18} />
                  <span>
                    <strong>
                      {project.nodes.find((n) => n.id === task.ownerId)
                        ?.title ??
                        (task.kind === "image" ? t("图片生成") : t("视频生成"))}
                    </strong>
                    <small>{task.instruction || task.prompt}</small>
                    <span
                      className={
                        task.status === "FAILED"
                          ? "task-failed"
                          : pendingRun(task)
                            ? "task-active"
                            : ""
                      }
                    >
                      {t(runStatuses[task.status ?? ""] ?? "") || t("待开始")}
                      {task.hiddenFromList ? t(" · 已移除") : ""}
                    </span>
                  </span>
                </button>
              );
            })}
          </div>
          <div className="task-panel-detail">
            {selected ? (
              <>
                <div className="task-panel-record-actions">
                  <button
                    type="button"
                    onClick={() => {
                      canvas.hideTask(selected.key, !selected.hiddenFromList);
                      if (!selected.hiddenFromList) canvas.clearActiveTask();
                    }}
                  >
                    {selected.hiddenFromList
                      ? t("恢复到列表")
                      : t("从列表移除")}
                  </button>
                </div>
                <GenerationRun
                  key={selected.key}
                  task={selected}
                  project={project}
                  canvas={canvas}
                  settings={settings}
                  follow={() => canvas.referenceTask(selected)}
                />
              </>
            ) : (
              <p>
                {t(
                  runs.length
                    ? "任务记录与生成素材均已保留。"
                    : "尚未创建生成任务。Agent 提交失败时不会产生任务记录。",
                )}
              </p>
            )}
          </div>
        </div>
      </Dialog.Content>
    </Dialog.Root>
  );
}
