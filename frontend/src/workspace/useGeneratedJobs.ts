import { errorText, normalizeError, issue } from "../errors/catalog";
import { retryDelay } from "../production/recovery";
import { useEffect, useRef } from "react";
import { native } from "../bridge";
import { runtime } from "../plugins/runtime";
import { receiveGeneratedResult } from "../creative/receiveGeneratedResult";
import type { Asset, Project } from "../model";
import {
  needsTracking,
  terminalOutputCount,
  type GeneratedJob as Job,
} from "./generatedJobTracking";
import { jobStatus, saveTask } from "../production/document";
export function useGeneratedJobs(
  projectId: string,
  get: () => Project,
  change: (f: (p: Project) => Project, record?: boolean) => void,
  flush: () => Promise<void>,
  error: (message: string) => void,
) {
  const latest = useRef({ change, flush, error, get });
  latest.current = { change, flush, error, get };
  useEffect(() => {
    if (!native) return;
    const pending = new Set<string>();
    const retries = new Map<string, { attempt: number; after: number }>();
    let stopped = false,
      working = false;
    async function poll() {
      if (stopped || working) return;
      working = true;
      try {
        const jobs = await runtime.execute<Job[]>("list_jobs", { projectId });
        if (stopped) return;
        for (const job of jobs) {
          if (needsTracking(job, latest.current.get())) pending.add(job.id);
        }
        for (let job of jobs.filter((j) => pending.has(j.id))) {
          if (stopped) break;
          if ((retries.get(job.id)?.after ?? 0) > Date.now()) continue;
          try {
            if (
              [
                "IN_QUEUE",
                "IN_PROGRESS",
                "CANCEL_REQUESTED",
                "UNKNOWN",
                "SUBMITTING",
              ].includes(job.status) &&
              job.requestId
            )
              job = await runtime.execute<Job>("refresh_job", { id: job.id });
            if (stopped) break;
            latest.current.change(
              (p) =>
                jobStatus(
                  p,
                  job.id,
                  terminalOutputCount(job) > 0 ? "RECEIVING" : job.status,
                  job.error,
                  job.progress,
                  job.requestId,
                ),
              false,
            );
            if (terminalOutputCount(job) > 0) {
              for (let index = 0; index < terminalOutputCount(job); index++) {
                const asset = await runtime.execute<Asset>(
                  "import_job_result",
                  { id: job.id, index },
                );
                if (stopped) return;
                latest.current.change((p) =>
                  receiveGeneratedResult(p, asset, job.shot),
                );
                await latest.current.flush();
              }
              latest.current.change(
                (p) =>
                  jobStatus(p, job.id, job.status, job.error, job.progress),
                false,
              );
              await latest.current.flush();
              pending.delete(job.id);
              window.dispatchEvent(new Event("studio-job-updated"));
            } else if (["FAILED", "CANCELLED"].includes(job.status)) {
              pending.delete(job.id);
            }
            retries.delete(job.id);
          } catch (e) {
            const attempt = (retries.get(job.id)?.attempt ?? 0) + 1;
            const parsed = normalizeError(e, "JOB_SYNC_FAILED");
            const delay = retryDelay(attempt);
            const retryable =
              terminalOutputCount(job) > 0 ||
              parsed.retryable ||
              ["NETWORK_ERROR", "JOB_SYNC_FAILED"].includes(parsed.code);
            retries.set(job.id, {
              attempt,
              after:
                retryable && delay !== undefined
                  ? Date.now() + delay
                  : Infinity,
            });
            latest.current.change((p) => {
              const task = Object.values(p.production?.drafts ?? {}).find(
                (t) => t.jobId === job.id,
              );
              const updated = jobStatus(
                p,
                job.id,
                terminalOutputCount(job) > 0
                  ? "RECEIVING"
                  : (task?.status ?? job.status),
                terminalOutputCount(job) > 0
                  ? JSON.stringify(
                      issue("RESULT_IMPORT_FAILED", parsed.details, {
                        stage: "import",
                      }),
                    )
                  : errorText(e, "JOB_SYNC_FAILED"),
                task?.progress,
              );
              const current = task && updated.production?.drafts?.[task.key];
              return current
                ? saveTask(updated, {
                    ...current,
                    trackingPaused: !retryable || delay === undefined,
                  })
                : updated;
            }, false);
          }
        }
        window.dispatchEvent(new Event("studio-job-updated"));
      } catch (e) {
        latest.current.error(errorText(e, "JOB_SYNC_FAILED"));
      } finally {
        working = false;
      }
    }
    const submitted = (event: Event) => {
      const detail = (event as CustomEvent<{ projectId: string; id: string }>)
        .detail;
      if (detail.projectId !== projectId || !detail.id) return;
      pending.add(detail.id);
      retries.delete(detail.id);
      void poll();
    };
    window.addEventListener("studio-job-submitted", submitted);
    const timer = window.setInterval(() => void poll(), 4000);
    void poll();
    return () => {
      stopped = true;
      clearInterval(timer);
      window.removeEventListener("studio-job-submitted", submitted);
    };
  }, [projectId]);
}
