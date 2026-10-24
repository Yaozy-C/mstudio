import { t, useLanguage } from "../i18n";
import { useState } from "react";
import { normalizeTaskTitle } from "./taskTitle";
import "./task-title-field.css";

export function TaskTitleField({
  value,
  change,
  commitOnBlur = false,
}: {
  value: string;
  change: (value: string) => void;
  commitOnBlur?: boolean;
}) {
  useLanguage();
  const [draft, setDraft] = useState<string | null>(null);
  function commit() {
    if (commitOnBlur && draft !== null)
      change(normalizeTaskTitle(draft, t("未命名")));
    else if (!commitOnBlur) change(normalizeTaskTitle(value, t("未命名")));
    setDraft(null);
  }
  return (
    <>
      <label className="task-title-field">
        <span>{t("任务名称")}</span>
        <input
          type="text"
          value={draft ?? value}
          placeholder={t("例如：小满 · 人物资产、海滩 · 黄昏场景")}
          onChange={(event) => {
            if (commitOnBlur) setDraft(event.target.value);
            else change(event.target.value);
          }}
          onBlur={commit}
          onKeyDown={(event) => {
            if (
              commitOnBlur &&
              event.key === "Enter" &&
              !event.nativeEvent.isComposing
            ) {
              event.preventDefault();
              commit();
            }
          }}
        />
      </label>
    </>
  );
}
