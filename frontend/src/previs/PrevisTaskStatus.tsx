import { Check, Cube } from "@phosphor-icons/react";
import { t } from "../i18n";
import { StatusIcon } from "../ui/AsyncState";
import type { PrevisEditorState } from "./usePrevisEditor";

export function previsTaskLabel(
  state: Pick<PrevisEditorState, "phase" | "stopping" | "error" | "lastKind">,
) {
  if (state.stopping) return t("正在停止预演…");
  if (state.error) return t("预演生成未完成");
  switch (state.phase) {
    case "saving":
      return t("正在保存本次要求…");
    case "generating":
      return t("正在生成预演场景…");
    case "loading":
      return t("正在加载三维场景…");
    case "rendering":
      return t("正在渲染预演画面…");
    case "encoding":
      return t("正在编码预演视频…");
    case "complete":
      return state.lastKind === "render"
        ? t("预演视频已保存")
        : t("预演已更新");
    case "cancelled":
      return t("预演任务已停止");
    default:
      return t("预演任务");
  }
}
export function PrevisTaskStatus({
  state,
  stage = false,
}: {
  state: PrevisEditorState;
  stage?: boolean;
}) {
  const { phase, frames, seconds, stopping, lastKind } = state;
  const step = phase === "rendering" ? 1 : phase === "encoding" ? 2 : 0;
  return (
    <div
      className={`previs-task-status${stage ? " previs-task-stage" : ""}`}
      role="status"
      aria-atomic="true"
    >
      {stage && (
        <Cube className="previs-task-cube" size={36} aria-hidden="true" />
      )}
      <div className="previs-task-title">
        <StatusIcon kind="loading" />
        <strong>{previsTaskLabel(state)}</strong>
        <time>{t("已用时 {seconds} 秒", { seconds })}</time>
      </div>
      <p>{t("可关闭此窗口，任务将在后台继续。")}</p>
      {lastKind === "render" && !stopping && (
        <ol className="previs-task-steps" aria-label={t("预演生成步骤")}>
          {[t("构建场景"), t("渲染画面"), t("保存视频")].map((label, i) => (
            <li
              key={i}
              data-state={i < step ? "done" : i === step ? "active" : "pending"}
            >
              {i < step ? (
                <Check size={14} aria-hidden="true" />
              ) : (
                <span>{i + 1}</span>
              )}
              {label}
            </li>
          ))}
        </ol>
      )}
      {phase === "rendering" && frames.total > 0 && (
        <div className="previs-task-progress">
          <progress
            value={frames.done}
            max={frames.total}
            aria-label={t("预演渲染进度")}
          />
          <small>{t("正在渲染 {done} / {total} 帧", frames)}</small>
        </div>
      )}
    </div>
  );
}
