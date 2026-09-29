import { errorText, issue } from "../errors/catalog";
import { pendingStatuses } from "./recovery";
import { useState } from "react";
import { runtime } from "../plugins/runtime";
import type { ProductionTask } from "./types";
import type { ProductionController } from "./useProduction";

export const runStatuses: Record<string, string> = {
  READY: "准备执行",
  AWAITING_CONFIRMATION: "待开始",
  UPLOADING: "正在准备素材",
  SUBMITTING: "正在提交",
  IN_QUEUE: "排队中",
  IN_PROGRESS: "处理中",
  COMPLETED: "已完成",
  RECEIVING: "正在收取结果",
  CANCELLED: "已取消",
  FAILED: "生成失败",
  CANCEL_REQUESTED: "正在取消",
  UNKNOWN: "正在核实提交状态",
};
export const pendingRun = (t: ProductionTask) =>
  pendingStatuses.includes(t.status ?? "");
export function useRunActions(
  task: ProductionTask,
  canvas: ProductionController,
  projectId: string,
) {
  const [busy, setBusy] = useState(false);
  async function check(cancel = false) {
    if (!task.jobId) {
      if (cancel)
        canvas.update({ status: "CANCELLED", error: undefined }, task.key);
      return;
    }
    setBusy(true);
    try {
      type Job = {
        id: string;
        status: string;
        outputCount?: number;
        requestId?: string;
        error?: string;
        progress?: ProductionTask["progress"];
      };
      const jobs = await runtime.execute<Job[]>("list_jobs", { projectId });
      let job = jobs.find((j) => j.id === task.jobId);
      if (!job) throw new Error(JSON.stringify(issue("JOB_NOT_FOUND")));
      if (
        cancel ||
        job.requestId ||
        ["COMPLETED", "FAILED", "CANCELLED"].includes(job.status)
      )
        job = await runtime.execute<Job>(
          cancel ? "cancel_job" : "refresh_job",
          { id: job.id },
        );
      canvas.update(
        {
          status:
            ["COMPLETED", "FAILED", "CANCELLED"].includes(job.status) &&
            (job.outputCount ?? (job.status === "COMPLETED" ? 1 : 0)) >
              (
                task.resultAssetIds ??
                (task.resultAssetId ? [task.resultAssetId] : [])
              ).length
              ? "RECEIVING"
              : job.status,
          requestId: job.requestId,
          error: job.error,
          trackingPaused: false,
          progress: job.progress,
        },
        task.key,
      );
      window.dispatchEvent(
        new CustomEvent("studio-job-submitted", {
          detail: { projectId, id: job.id },
        }),
      );
    } catch (error) {
      canvas.update(
        {
          error: errorText(
            error,
            cancel ? "JOB_CANCEL_FAILED" : "JOB_SYNC_FAILED",
          ),
        },
        task.key,
      );
    } finally {
      setBusy(false);
    }
  }
  return { busy, check };
}
