import { t } from "../i18n";
import { StatusIcon, type StatusKind } from "../ui/AsyncState";
import type { ProductionTask } from "./types";
import { pendingRun, runStatuses } from "./useRunActions";

export function runStatusKind(task: ProductionTask): StatusKind {
  if (task.status === "COMPLETED") return "success";
  if (task.status === "FAILED") return "error";
  if (
    task.trackingPaused ||
    task.status === "UNKNOWN" ||
    task.status === "CANCELLED"
  )
    return "paused";
  return pendingRun(task) ? "loading" : "info";
}
export function RunStatus({ task }: { task: ProductionTask }) {
  const kind = runStatusKind(task);
  return (
    <span className="run-status" data-kind={kind} role="status">
      <StatusIcon kind={kind} size={14} />
      {task.trackingPaused
        ? t("状态同步已暂停")
        : task.status === "UNKNOWN"
          ? t("待核查")
          : t(runStatuses[task.status ?? ""] ?? "") || t("待开始")}
    </span>
  );
}
