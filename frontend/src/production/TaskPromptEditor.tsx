import { useState } from "react";
import type { ProductionTask } from "./types";
import type { ProductionController } from "./useProduction";
import { canEditOriginal } from "./taskEditing";

export function TaskPromptEditor({
  task,
  canvas,
}: {
  task: ProductionTask;
  canvas: ProductionController;
}) {
  const original = canEditOriginal(task);
  const saved = task.nextPrompt ?? task.prompt;
  const [draft, setDraft] = useState<string | null>(null);
  const [error, setError] = useState("");
  return (
    <details className="run-description task-prompt-editor">
      <summary>
        生成描述{task.nextPrompt !== undefined ? " · 已有修改" : ""}
      </summary>
      {!original && (
        <details>
          <summary>查看本次原始描述</summary>
          <p>{task.prompt}</p>
        </details>
      )}
      <label>
        {original ? "生成描述" : "下次生成描述"}
        <textarea
          aria-label={original ? "生成描述" : "下次生成描述"}
          rows={5}
          maxLength={12000}
          value={draft ?? saved}
          onChange={(e) => {
            setDraft(e.target.value);
            setError("");
          }}
        />
      </label>
      <div className="run-actions">
        <button
          type="button"
          disabled={draft === null || !draft.trim()}
          onClick={() => {
            try {
              canvas.editPrompt(task.key, draft!);
              setDraft(null);
            } catch (e) {
              setError(String(e).replace(/^Error: /, ""));
            }
          }}
        >
          保存描述
        </button>
        {draft !== null && (
          <button
            type="button"
            onClick={() => {
              setDraft(null);
              setError("");
            }}
          >
            放弃修改
          </button>
        )}
        <button
          type="button"
          disabled={draft !== null && !draft.trim()}
          onClick={() => {
            try {
              if (draft !== null) {
                canvas.editPrompt(task.key, draft);
                setDraft(null);
              }
              canvas.referenceTask(task);
            } catch (e) {
              setError(String(e).replace(/^Error: /, ""));
            }
          }}
        >
          让 Agent 修改描述
        </button>
      </div>
      {error && <p role="alert">{error}</p>}
      {!original && (
        <small>保存后不会改变当前请求；重新生成会创建新任务。</small>
      )}
    </details>
  );
}
