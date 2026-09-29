import { useContext, useState } from "react";
import { t, useLanguage } from "../i18n";
import { MessageContext } from "../assistant/AgentMessage";
import { batchSummary } from "./batch";
import { TaskEntry } from "./TaskEntry";
const PAGE_SIZE = 12;
export function GenerationMessages({ turnId }: { turnId: string }) {
  useLanguage();
  const context = useContext(MessageContext)!;
  const [page, setPage] = useState(0);
  const tasks = context.canvas?.runs.filter((t) => t.turnId === turnId) ?? [];
  if (!tasks.length) return null;
  const current = Math.min(page, Math.ceil(tasks.length / PAGE_SIZE) - 1);
  const summary = batchSummary(tasks);
  return (
    <div className="conversation-generation-results">
      {tasks.length > 1 && (
        <div className="generation-batch">
          <p role="status">
            {t("共 {total} 个任务 · 完成 {completed} · 需处理 {attention}", {
              total: summary.total,
              completed: summary.counts.COMPLETED ?? 0,
              attention: summary.attention,
            })}
          </p>
          {tasks.some(
            (task) =>
              task.status === "AWAITING_CONFIRMATION" &&
              task.modelId &&
              !task.jobId &&
              !task.submissionId,
          ) && (
            <button
              type="button"
              onClick={() => context.canvas?.batch(turnId, "resume")}
            >
              {t("继续")}
            </button>
          )}
          {!!summary.counts.READY && (
            <button
              type="button"
              onClick={() => context.canvas?.batch(turnId, "stop")}
            >
              {t("停止")}
            </button>
          )}
          {tasks.some(
            (task) =>
              task.status === "FAILED" &&
              task.modelId &&
              !task.resultAssetId &&
              !task.resultAssetIds?.length,
          ) && (
            <button
              type="button"
              onClick={() => context.canvas?.batch(turnId, "retry")}
            >
              {t("重试失败")}
            </button>
          )}
        </div>
      )}
      {context.canvas &&
        tasks
          .slice(current * PAGE_SIZE, (current + 1) * PAGE_SIZE)
          .map((task) => (
            <TaskEntry
              key={task.key}
              task={task}
              canvas={context.canvas!}
              title={
                context.project.nodes.find(
                  (n) => n.id === (task.targetNodeId ?? task.ownerId),
                )?.title
              }
            />
          ))}
      {tasks.length > PAGE_SIZE && (
        <nav className="generation-pagination" aria-label={t("生成任务分页")}>
          <button
            type="button"
            disabled={current === 0}
            onClick={() => setPage(current - 1)}
          >
            {t("上一页")}
          </button>
          <span>
            {" "}
            {current + 1} / {Math.ceil(tasks.length / PAGE_SIZE)}{" "}
          </span>
          <button
            type="button"
            disabled={(current + 1) * PAGE_SIZE >= tasks.length}
            onClick={() => setPage(current + 1)}
          >
            {t("下一页")}
          </button>
        </nav>
      )}
    </div>
  );
}
