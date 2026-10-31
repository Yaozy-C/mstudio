import { referenceAsset } from "../previs/media";
import { RunStatus } from "./RunStatus";
import { t, useLanguage } from "../i18n";
import { GenerationSettings } from "./GenerationSettings";
import { useState } from "react";
import { TaskPromptEditor } from "./TaskPromptEditor";
import { canEditOriginal, canRegenerate } from "./taskEditing";
import { normalizeError } from "../errors/catalog";
import { ErrorNotice } from "../errors/ErrorNotice";
import { runProgress } from "./runProgress";
import { taskTitle } from "./taskTitle";
import {
  Image,
  VideoCamera,
  ArrowClockwise,
  SlidersHorizontal,
  X,
  Stop,
  PlugsConnected,
} from "@phosphor-icons/react";
import { native, mediaUrl } from "../bridge";
import type { Project } from "../model";
import type { ProductionTask } from "./types";
import type { ProductionController } from "./useProduction";
import { canvasSnapshot, inputFor, roleLabels } from "./request";
import { MediaPreview } from "../workspace/MediaPreview";
import { GenerationResults } from "./GenerationResults";
import type { ReactNode } from "react";
import { ActionButton } from "../ui/ActionButton";
import { pendingRun, useRunActions } from "./useRunActions";
import "./conversation.css";
import "./task-detail.css";

export function GenerationRun({
  task,
  project,
  canvas,
  settings,
  headerAction,
}: {
  task: ProductionTask;
  project: Project;
  canvas: ProductionController;
  settings: () => void;
  headerAction?: ReactNode;
  follow: (
    task: ProductionTask,
    text?: string,
    kind?: "image" | "video",
  ) => void;
}) {
  useLanguage();
  const [preview, setPreview] = useState<string | null>(null);
  const [startError, setStartError] = useState("");
  const { busy, check } = useRunActions(task, canvas, project.id);
  const pending = pendingRun(task);
  const editable = !pending && (canEditOriginal(task) || canRegenerate(task));
  const models = canvas.media.models.filter((m) => m.kind === task.kind);
  const model = models.find((m) => m.id === task.modelId);
  const node = project.nodes.find(
    (n) => n.id === (task.targetNodeId ?? task.ownerId),
  );
  const title = taskTitle(task, t("未命名"));
  let issue = "";
  if (editable)
    try {
      canvasSnapshot(project, task, { x: 0, y: 0 });
      if (model) inputFor(project, task, model);
    } catch (error) {
      issue = normalizeError(error, "VALIDATION_FAILED").message;
    }
  const Icon = task.kind === "image" ? Image : VideoCamera;
  return (
    <section
      className="generation-run"
      aria-label={t("生成{v0}任务", {
        v0: task.kind === "image" ? t("图片") : t("视频"),
      })}
    >
      <div className="run-content">
        <header>
          <Icon size={17} />
          <strong>{title}</strong>
          <RunStatus task={task} />
          {headerAction}
        </header>
        {node && node.title !== title && (
          <small className="run-owner">{node.title}</small>
        )}
        <p className="run-instruction">{task.instruction || task.prompt}</p>
        {!editable && (
          <div className="run-metadata">
            <span>{model?.name ?? t("所选模型")}</span>
          </div>
        )}
        <GenerationResults task={task} project={project} canvas={canvas} />
        {editable ? (
          <GenerationSettings
            task={task}
            project={project}
            models={models}
            close={() => {}}
            save={() => {}}
            onPatch={(patch) => {
              canvas.update(patch, task.key);
              setStartError("");
            }}
          />
        ) : (
          <div className="run-references" aria-label={t("本次使用素材")}>
            {task.inputs
              .filter((r) => r.assetId)
              .map((r) => {
                const a = referenceAsset(project, r.assetId);
                return (
                  <button
                    type="button"
                    key={r.key}
                    title={`${a?.name ?? t("素材已移除")} · ${t(roleLabels[r.role])}`}
                    aria-label={t("预览 {name}", {
                      name: a?.name ?? t("素材"),
                    })}
                    onClick={() => setPreview(r.assetId)}
                  >
                    {a?.preview && (
                      <img src={mediaUrl(a.preview)} alt={a.name} />
                    )}
                    <small>
                      {t(roleLabels[r.role])}
                      {r.start !== undefined && r.end !== undefined
                        ? ` ${r.start}–${r.end}s`
                        : ""}
                    </small>
                  </button>
                );
              })}
          </div>
        )}
        <TaskPromptEditor key={task.key} task={task} canvas={canvas} />
      </div>
      <MediaPreview
        asset={project.assets.find((a) => a.id === preview)}
        close={() => setPreview(null)}
      />
      <footer className="run-footer">
        {editable && !models.length && (
          <ActionButton icon={PlugsConnected} onClick={settings}>
            {t("连接生成模型")}
          </ActionButton>
        )}
        {startError && (
          <ErrorNotice error={startError} fallback="VALIDATION_FAILED" />
        )}
        {issue && (
          <p className="run-notice" role="status">
            {issue}
          </p>
        )}
        <ErrorNotice
          error={task.error}
          fallback={
            ["FAILED", "UNKNOWN"].includes(task.status ?? "")
              ? "GENERATION_FAILED"
              : "JOB_SYNC_FAILED"
          }
          taskId={task.requestId ?? task.jobId}
        >
          {[
            "INVALID_INPUT",
            "AUTH_REQUIRED",
            "ACCESS_DENIED",
            "QUOTA_EXCEEDED",
            "MODEL_UNAVAILABLE",
          ].includes(normalizeError(task.error).code) && (
            <ActionButton icon={SlidersHorizontal} onClick={settings}>
              {t("模型与连接设置")}
            </ActionButton>
          )}
        </ErrorNotice>
        {editable && (
          <div className="run-actions">
            <ActionButton
              icon={ArrowClockwise}
              type="button"
              disabled={!native || busy || !!issue || !model?.enabled}
              onClick={async () => {
                try {
                  setStartError("");
                  if (canRegenerate(task)) canvas.retry(task);
                  else {
                    if (task.status === "DRAFT")
                      canvas.update(
                        { status: "AWAITING_CONFIRMATION" },
                        task.key,
                      );
                    await canvas.submit(task);
                  }
                } catch (error) {
                  setStartError(String(error));
                }
              }}
            >
              {canRegenerate(task)
                ? task.status === "COMPLETED"
                  ? t("重新生成")
                  : t("重试")
                : t("生成")}
            </ActionButton>
            {canEditOriginal(task) && (
              <ActionButton
                icon={X}
                type="button"
                disabled={busy}
                onClick={() => void check(true)}
              >
                {t("取消")}
              </ActionButton>
            )}
          </div>
        )}
        {pending && (
          <div className="run-actions">
            <div className="run-progress" role="status" aria-live="polite">
              <span>{t(runProgress(task))}</span>
              {task.error && (
                <small>
                  {task.trackingPaused
                    ? t(" · 自动重试已暂停，请处理后手动重试")
                    : t(" · 正在重试原任务，不会重新生成")}
                </small>
              )}
            </div>
            {task.jobId && task.error && (
              <ActionButton
                icon={ArrowClockwise}
                busy={busy}
                busyLabel={t("正在核查…")}
                onClick={() => void check()}
              >
                {task.status === "RECEIVING"
                  ? t("重试收取结果")
                  : t("重新查询状态")}
              </ActionButton>
            )}
            {["READY", "IN_QUEUE", "IN_PROGRESS"].includes(
              task.status ?? "",
            ) && (
              <ActionButton
                icon={Stop}
                type="button"
                disabled={busy}
                onClick={() => void check(true)}
              >
                {t("停止生成")}
              </ActionButton>
            )}
          </div>
        )}
      </footer>
    </section>
  );
}
