import { FloppyDisk, ChatText, CaretDown, X } from "@phosphor-icons/react";
import { t, useLanguage } from "../i18n";
import { useState } from "react";
import { ActionButton } from "../ui/ActionButton";
import type { ProductionTask } from "./types";
import type { ProductionController } from "./useProduction";

export function TaskPromptEditor({
  task,
  canvas,
}: {
  task: ProductionTask;
  canvas: ProductionController;
}) {
  useLanguage();
  const saved = task.prompt;
  const [draft, setDraft] = useState<string | null>(null);
  const [error, setError] = useState("");
  return (
    <details className="run-description task-prompt-editor">
      <summary>
        <CaretDown size={16} />
        {t("生成描述")}
      </summary>
      <label>
        <textarea
          aria-label={t("生成描述")}
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
        <ActionButton
          icon={FloppyDisk}
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
          {t("保存描述")}
        </ActionButton>
        {draft !== null && (
          <ActionButton
            icon={X}
            type="button"
            onClick={() => {
              setDraft(null);
              setError("");
            }}
          >
            {t("放弃修改")}
          </ActionButton>
        )}
        <ActionButton
          type="button"
          icon={ChatText}
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
          {t("让 Agent 修改描述")}
        </ActionButton>
      </div>
      {error && <p role="alert">{error}</p>}
    </details>
  );
}
