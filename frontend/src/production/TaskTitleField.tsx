import { t, useLanguage } from "../i18n";
import { useState } from "react";
import { ErrorNotice } from "../errors/ErrorNotice";
import { checkedTaskTitle } from "./taskTitle";
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
  const [error, setError] = useState("");
  function commit() {
    try {
      if (commitOnBlur && draft !== null) change(checkedTaskTitle(draft));
      else if (!commitOnBlur && value !== value.trim()) change(value.trim());
      setError("");
    } catch (error) {
      setError(String(error).replace(/^Error: /, ""));
    }
    setDraft(null);
  }
  return (
    <>
      <label className="task-title-field">
        <span>{t("任务名称")}</span>
        <input
          type="text"
          required
          maxLength={100}
          value={draft ?? value}
          placeholder={t("例如：小满 · 人物资产、海滩 · 黄昏场景")}
          onChange={(event) => {
            if (commitOnBlur) setDraft(event.target.value);
            else change(event.target.value);
            setError("");
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
      {error && <ErrorNotice error={error} fallback="VALIDATION_FAILED" />}
    </>
  );
}
