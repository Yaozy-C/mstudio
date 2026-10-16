import { Cube } from "@phosphor-icons/react";
import { t } from "../i18n";
import { StatusIcon } from "../ui/AsyncState";
import { previsTaskLabel } from "./PrevisTaskStatus";
import type { PrevisPhase } from "./usePrevisTask";

export type PrevisTaskSummary = {
  fileId: string;
  name: string;
  running: boolean;
  phase: PrevisPhase;
  stopping: boolean;
  error: string;
  lastKind: "chat" | "render";
};
export function PrevisTaskList({
  tasks,
  open,
}: {
  tasks: PrevisTaskSummary[];
  open: (fileId: string) => void;
}) {
  if (!tasks.length) return null;
  return (
    <>
      <h3 className="previs-task-list-heading">{t("预演任务")}</h3>
      {tasks.map((task) => (
        <button
          type="button"
          key={task.fileId}
          onClick={() => open(task.fileId)}
        >
          <span className="task-thumbnail" aria-hidden="true">
            <Cube size={20} />
          </span>
          <span className="task-list-copy">
            <strong title={task.name}>{task.name}</strong>
            <span className="task-list-meta">
              <span
                className="run-status"
                data-kind={task.error ? "error" : undefined}
              >
                <StatusIcon
                  kind={
                    task.error
                      ? "error"
                      : task.running
                        ? "loading"
                        : task.phase === "cancelled"
                          ? "paused"
                          : "success"
                  }
                  size={16}
                />
                {previsTaskLabel(task)}
              </span>
            </span>
            <small>{t("查看预演")}</small>
          </span>
        </button>
      ))}
    </>
  );
}
