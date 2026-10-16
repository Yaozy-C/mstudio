import { useState } from "react";
import { Dialog } from "@radix-ui/themes";
import { t, useLanguage } from "../i18n";
import {
  ListChecks,
  X,
  Image,
  VideoCamera,
  Archive,
  ArrowCounterClockwise,
} from "@phosphor-icons/react";
import type { Project } from "../model";
import type { ProductionController } from "./useProduction";
import { GenerationRun } from "./GenerationRun";
import { pendingRun } from "./useRunActions";
import { RunStatus } from "./RunStatus";
import { mediaUrl } from "../bridge";
import "./task-panel.css";
import { useOptionalPrevis } from "../previs/context";
import { PrevisTaskList } from "../previs/PrevisTaskList";

export function TaskPanel({
  project,
  canvas,
  settings,
}: {
  project: Project;
  canvas: ProductionController;
  settings: () => void;
}) {
  const language = useLanguage();
  const [archived, setArchived] = useState(false);
  const previs = useOptionalPrevis();
  const previsTasks = archived ? [] : (previs?.tasks ?? []);
  const runs = [...canvas.runs].reverse();
  const visible = runs.filter(
    (t) => !!t.hiddenFromList === archived || t.key === canvas.activeTaskKey,
  );
  const selected =
    visible.find((t) => t.key === canvas.activeTaskKey) ?? visible[0];
  const active =
    visible.filter(pendingRun).length +
    previsTasks.filter((task) => task.running).length;
  const failed =
    visible.filter((t) => t.status === "FAILED").length +
    previsTasks.filter((task) => !!task.error).length;
  return (
    <Dialog.Root
      open={canvas.taskPanelOpen}
      onOpenChange={canvas.setTaskPanelOpen}
    >
      <Dialog.Trigger>
        <button type="button" className="chrome-action task-dialog-trigger">
          <ListChecks />
          {t("生成任务")} <span>{visible.length + previsTasks.length}</span>
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
          <button
            type="button"
            aria-pressed={archived}
            onClick={() => {
              setArchived(!archived);
              canvas.clearActiveTask();
            }}
          >
            {archived ? t("当前任务") : t("已归档")}
          </button>
          <Dialog.Close>
            <button type="button" aria-label={t("关闭生成任务")}>
              <X />
            </button>
          </Dialog.Close>
        </header>
        <div className="task-panel-body">
          <div className="task-panel-list" aria-label={t("任务列表")}>
            <PrevisTaskList
              tasks={previsTasks}
              open={(fileId) => {
                canvas.setTaskPanelOpen(false);
                previs?.open(fileId);
              }}
            />
            {!visible.length && !previsTasks.length && (
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
              const resultIds = new Set([
                task.resultAssetId,
                ...(task.resultAssetIds ?? []),
              ]);
              const thumbnail = project.assets.find(
                (asset) =>
                  resultIds.has(asset.id) &&
                  !asset.missing &&
                  !asset.discarded &&
                  !!asset.preview,
              );
              const owner = project.nodes.find(
                (node) => node.id === task.ownerId,
              );
              const title =
                owner?.title ||
                task.instruction ||
                (task.kind === "image" ? t("图片生成") : t("视频生成"));
              const date =
                task.createdAt && Number.isFinite(task.createdAt)
                  ? new Date(task.createdAt)
                  : null;
              return (
                <button
                  type="button"
                  key={task.key}
                  aria-pressed={selected?.key === task.key}
                  className={selected?.key === task.key ? "selected" : ""}
                  onClick={() => canvas.showTask(task)}
                >
                  <span className="task-thumbnail" aria-hidden="true">
                    {thumbnail ? (
                      <img
                        src={mediaUrl(thumbnail.preview)}
                        alt=""
                        loading="lazy"
                      />
                    ) : (
                      <Icon size={20} />
                    )}
                  </span>
                  <span className="task-list-copy">
                    <strong title={title}>{title}</strong>
                    {owner && <small>{task.instruction || task.prompt}</small>}
                    <span className="task-list-meta">
                      <RunStatus task={task} />
                      {date && (
                        <time
                          dateTime={date.toISOString()}
                          title={date.toLocaleString(language)}
                        >
                          {date.toLocaleString(language, {
                            month: "numeric",
                            day: "numeric",
                            hour: "2-digit",
                            minute: "2-digit",
                          })}
                        </time>
                      )}
                    </span>
                  </span>
                </button>
              );
            })}
          </div>
          <div className="task-panel-detail">
            {selected ? (
              <GenerationRun
                key={selected.key}
                task={selected}
                project={project}
                canvas={canvas}
                settings={settings}
                follow={() => canvas.referenceTask(selected)}
                headerAction={
                  <button
                    type="button"
                    className="task-record-action icon-button"
                    title={
                      selected.hiddenFromList ? t("恢复到列表") : t("归档任务")
                    }
                    aria-label={
                      selected.hiddenFromList ? t("恢复到列表") : t("归档任务")
                    }
                    disabled={!selected.hiddenFromList && pendingRun(selected)}
                    onClick={() => {
                      canvas.hideTask(selected.key, !selected.hiddenFromList);
                      if (!selected.hiddenFromList) canvas.clearActiveTask();
                    }}
                  >
                    {selected.hiddenFromList ? (
                      <ArrowCounterClockwise size={18} />
                    ) : (
                      <Archive size={18} />
                    )}
                  </button>
                }
              />
            ) : (
              <p>
                {t(
                  previsTasks.length
                    ? "选择预演任务，查看进度或继续修改。"
                    : runs.length
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
