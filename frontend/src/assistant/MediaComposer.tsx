import { normalizeError } from "../errors/catalog";
import { ErrorNotice } from "../errors/ErrorNotice";
import { compositionGuard } from "./compositionGuard";
import { inputFor } from "../production/request";
import { useState, useRef, useEffect } from "react";
import { ArrowUp } from "@phosphor-icons/react";
import type { Project } from "../model";
import type { ProductionController } from "../production/useProduction";
import { ComposerGeneration } from "../production/ComposerGeneration";
import { ComposerMediaInputs } from "./ComposerMediaInputs";
import { useComposerFiles } from "./useComposerFiles";
import type { AttachmentDraft } from "./useAttachments";
import "../styles/agent-composer.css";

export function MediaComposer({
  canvas,
  project,
  draft,
  settings,
}: {
  canvas: ProductionController;
  project: Project;
  draft: AttachmentDraft;
  settings: () => void;
}) {
  const [ime] = useState(compositionGuard);
  const [error, setError] = useState("");
  const submitting = useRef(false);
  const [preparing, setPreparing] = useState(false);
  const active = useRef(true);
  useEffect(() => {
    active.current = true;
    return () => {
      active.current = false;
    };
  }, []);
  const task = canvas.task;
  const model = canvas.media.models.find(
    (m) =>
      m.id ===
        (task?.modelId || (task && canvas.modelPreferences[task.kind])) &&
      m.enabled,
  );
  const { dragging, ...files } = useComposerFiles(draft, (refs) =>
    refs.forEach((ref) => canvas.attach(ref)),
  );
  let issue = "";
  if (task && model) {
    try {
      if (task.inputs.some((r) => r.role === "script"))
        throw new Error("文字资料请在 Agent 模式使用，或移除后生成");
      inputFor(project, { ...task, prompt: task.prompt || "验证素材" }, model);
    } catch (e) {
      issue = normalizeError(e, "VALIDATION_FAILED").message;
    }
  }
  const blocked =
    preparing || draft.busy || !model || !task?.prompt.trim() || !!issue;
  async function send() {
    if (blocked || submitting.current) return;
    submitting.current = true;
    setPreparing(true);
    try {
      await canvas.execute(() => active.current);
      setError("");
    } catch (e) {
      setError(String(e).replace(/^Error: /, ""));
    } finally {
      submitting.current = false;
      if (active.current) setPreparing(false);
    }
  }
  return (
    <div className="agent-input-area">
      {preparing && (
        <p className="agent-attachment-hint" role="status">
          正在理解素材并整理提示词…
        </p>
      )}
      {issue && (
        <p className="agent-attachment-hint" role="status">
          {issue}
        </p>
      )}
      {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
      {draft.notice && <p className="agent-attachment-hint">{draft.notice}</p>}
      <form
        className={`agent-composer media-composer${dragging ? " is-file-dragging" : ""}`}
        {...(preparing ? {} : files)}
        onSubmit={(e) => {
          e.preventDefault();
          send();
        }}
      >
        <fieldset
          disabled={preparing}
          inert={preparing}
          className="media-composer-fields"
        >
          {dragging && <div className="composer-drop-hint">松开以添加素材</div>}
          <ComposerMediaInputs
            canvas={canvas}
            project={project}
            draft={draft}
          />
          <textarea
            {...ime}
            aria-label={
              task?.kind === "video" ? "视频生成描述" : "图片生成描述"
            }
            rows={3}
            value={task?.prompt ?? ""}
            placeholder={
              task?.kind === "video"
                ? "描述动作、运镜和变化…"
                : "描述想生成的画面…"
            }
            onChange={(e) => canvas.update({ prompt: e.target.value })}
            onKeyDown={(e) => {
              if (
                e.key === "Enter" &&
                !e.shiftKey &&
                !e.nativeEvent.isComposing
              ) {
                e.preventDefault();
                send();
              }
            }}
          />
          <div className="agent-compose-actions">
            <div className="composer-tools">
              <ComposerGeneration
                canvas={canvas}
                project={project}
                settings={settings}
                running={preparing}
              />
            </div>
            <div className="composer-submit">
              <button
                type="submit"
                aria-label="开始生成"
                disabled={blocked}
                title={!model ? "先选择生成模型" : "开始生成"}
              >
                <ArrowUp size={18} weight="bold" />
              </button>
            </div>
          </div>
        </fieldset>
      </form>
    </div>
  );
}
