import { t, useLanguage } from "../i18n";
import { TaskPromptEditor } from "./TaskPromptEditor";
import { canEditOriginal, canRegenerate } from "./taskEditing";
import { UnknownRunRecovery } from "./UnknownRunRecovery";
import { normalizeError } from "../errors/catalog";
import { ErrorNotice } from "../errors/ErrorNotice";
import { runProgress } from "./runProgress";
import { Image, VideoCamera } from "@phosphor-icons/react";
import { native, mediaUrl } from "../bridge";
import type { Project } from "../model";
import type { ProductionTask } from "./types";
import type { ProductionController } from "./useProduction";
import { canvasSnapshot, inputFor, roleLabels } from "./request";
import { GenerationResult } from "./GenerationResult";
import { pendingRun, runStatuses, useRunActions } from "./useRunActions";
import "./conversation.css";

export function GenerationRun({
  task,
  project,
  canvas,
  settings,
}: {
  task: ProductionTask;
  project: Project;
  canvas: ProductionController;
  settings: () => void;
  follow: (
    task: ProductionTask,
    text?: string,
    kind?: "image" | "video",
  ) => void;
}) {
  useLanguage();
  const { busy, check } = useRunActions(task, canvas, project.id);
  const pending = pendingRun(task);
  const editable = !pending && (canEditOriginal(task) || canRegenerate(task));
  const models = canvas.media.models.filter((m) => m.kind === task.kind);
  const model = models.find((m) => m.id === task.modelId);
  const node = project.nodes.find((n) => n.id === task.ownerId);
  let issue = "";
  if (editable)
    try {
      canvasSnapshot(project, task, { x: 0, y: 0 });
      if (model) inputFor(project, task, model);
    } catch (error) {
      issue = normalizeError(error, "VALIDATION_FAILED").message;
    }
  const resultIds =
    task.resultAssetIds ?? (task.resultAssetId ? [task.resultAssetId] : []);
  const Icon = task.kind === "image" ? Image : VideoCamera;
  return (
    <section
      className="generation-run"
      aria-label={t("生成{v0}任务", {
        v0: task.kind === "image" ? t("图片") : t("视频"),
      })}
    >
      <header>
        <Icon size={17} />
        <strong>
          {t("生成")}
          {task.kind === "image" ? t("图片") : t("视频")}
        </strong>
        <span role="status" className={pending ? "run-active" : ""}>
          {t(runStatuses[task.status ?? ""] ?? "") || t("待开始")}
        </span>
      </header>
      {node && <small className="run-owner">{node.title}</small>}
      <p className="run-instruction">{task.instruction || task.prompt}</p>
      <div className="run-metadata">
        {canEditOriginal(task) ? (
          <select
            aria-label={t("本次生成模型")}
            value={task.modelId}
            onChange={(e) => {
              canvas.update(
                { modelId: e.target.value, error: undefined },
                task.key,
              );
              if (!canvas.modelPreferences[task.kind])
                canvas.preferences({ [task.kind]: e.target.value });
            }}
          >
            <option value="">
              {models.length ? t("选择模型") : t("尚未连接生成模型")}
            </option>
            {task.modelId && !model && (
              <option value={task.modelId}>{t("原模型不可用")}</option>
            )}
            {models.map((m) => (
              <option key={m.id} value={m.id}>
                {m.name}
              </option>
            ))}
          </select>
        ) : (
          <span>{model?.name ?? t("所选模型")}</span>
        )}
        <span>
          {task.kind === "image"
            ? t("图片")
            : {
                single: t("单图"),
                ends: t("首尾帧"),
                multi: t("多图参考"),
                mixed: t("视频与图片参考"),
              }[task.mode]}
        </span>
      </div>
      <div className="run-references" aria-label={t("本次使用素材")}>
        {task.inputs
          .filter((r) => r.assetId)
          .map((r) => {
            const a = project.assets.find((a) => a.id === r.assetId);
            return (
              <button
                type="button"
                key={r.key}
                title={`${a?.name ?? t("素材已移除")} · ${t(roleLabels[r.role])}`}
                aria-label={t("在画布查看 {v0}", { v0: a?.name ?? t("素材") })}
                onClick={() => {
                  const item = canvas.items.find(
                    (n) => n.assetId === r.assetId,
                  );
                  if (item) {
                    canvas.choose([item.key]);
                    canvas.focusShot(item.ownerId);
                  }
                }}
              >
                {a?.preview && <img src={mediaUrl(a.preview)} alt={a.name} />}
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
      <TaskPromptEditor key={task.key} task={task} canvas={canvas} />
      {resultIds.length > 0 && task.kind === "image" && (
        <p role="status">
          {t("已生成")} {resultIds.length} {t("张图片")}
        </p>
      )}
      {resultIds.map((resultId) => (
        <GenerationResult
          key={resultId}
          task={{ ...task, resultAssetId: resultId }}
          project={project}
          reference={(id) => canvas.attach({ kind: "asset", id })}
          locate={() => {
            const item = canvas.items.find((n) => n.assetId === resultId);
            if (item) canvas.focusItem(item.key);
          }}
        />
      ))}
      {editable && !models.length && (
        <button type="button" onClick={settings}>
          {t("连接生成模型")}
        </button>
      )}
      {issue && (
        <p className="run-notice" role="status">
          {issue}
        </p>
      )}
      <ErrorNotice
        error={task.error}
        fallback={
          task.status === "FAILED" ? "GENERATION_FAILED" : "JOB_SYNC_FAILED"
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
          <button type="button" onClick={settings}>
            {t("模型与连接设置")}
          </button>
        )}
      </ErrorNotice>
      {editable && (
        <div className="run-actions">
          <button
            type="button"
            className="primary"
            disabled={!native || busy}
            onClick={() => canvas.configure(task)}
          >
            {canRegenerate(task) ? t("重新设置并生成") : t("设置并生成")}
          </button>
          {canEditOriginal(task) && (
            <button
              type="button"
              disabled={busy}
              onClick={() => void check(true)}
            >
              {t("取消")}
            </button>
          )}
        </div>
      )}
      {pending && (
        <div className="run-actions">
          <div role="status" aria-live="polite">
            <span>{t(runProgress(task))}</span>
            {task.error && (
              <small>
                {task.status === "UNKNOWN"
                  ? t(" · 核查原任务后再继续")
                  : task.trackingPaused
                    ? t(" · 自动重试已暂停，请处理后手动重试")
                    : t(" · 正在重试原任务，不会重新生成")}
              </small>
            )}
          </div>
          {task.status === "UNKNOWN" && (
            <UnknownRunRecovery task={task} canvas={canvas} />
          )}
          {task.jobId && (task.status === "UNKNOWN" || task.error) && (
            <button type="button" disabled={busy} onClick={() => void check()}>
              {busy
                ? t("正在核查…")
                : task.status === "RECEIVING"
                  ? t("重试收取结果")
                  : t("重新查询状态")}
            </button>
          )}
          {["READY", "IN_QUEUE", "IN_PROGRESS"].includes(task.status ?? "") && (
            <button
              type="button"
              disabled={busy}
              onClick={() => void check(true)}
            >
              {t("停止生成")}
            </button>
          )}
        </div>
      )}
      {pending && task.nextPrompt !== undefined && canRegenerate(task) && (
        <button
          type="button"
          disabled={!native || busy}
          onClick={() => canvas.configure(task)}
        >
          {t("使用修改后的描述重新生成")}
        </button>
      )}
      {editable && (
        <small className="run-cost">
          {native
            ? t("使用所选服务生成，按服务计费。")
            : t("浏览器仅预览；请在桌面应用中生成。")}
        </small>
      )}
    </section>
  );
}
