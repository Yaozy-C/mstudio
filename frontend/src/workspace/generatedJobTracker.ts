import { errorText, normalizeError, issue } from "../errors/catalog";
import { retryDelay } from "../production/recovery";
import { workQueue } from "../runtime/workQueue";
import { receiveGeneratedResult } from "../creative/receiveGeneratedResult";
import type { Asset, Project } from "../model";
import {
  needsTracking,
  terminalOutputCount,
  type GeneratedJob as Job,
} from "./generatedJobTracking";
import { jobStatus, saveTask } from "../production/document";
export type JobTrackerHost = {
  get: () => Project;
  change: (fn: (p: Project) => Project, record?: boolean) => void;
  flush: () => Promise<void>;
  error: (message: string) => void;
  notify: () => void;
  execute: <T>(command: string, args: Record<string, unknown>) => Promise<T>;
};
export function generatedJobTracker(projectId: string, host: JobTrackerHost) {
  const queries = workQueue(4);
  const imports = workQueue(2);
  const retries = new Map<string, { attempt: number; after: number }>();
  let stopped = false,
    listing = false;
  const notify = host.notify;
  function failed(job: Job, e: unknown, stage: "status" | "import") {
    if (stopped) return;
    const key = `${stage}:${job.id}`;
    const attempt = (retries.get(key)?.attempt ?? 0) + 1;
    const parsed = normalizeError(e, "JOB_SYNC_FAILED");
    const delay = retryDelay(attempt);
    const retryable =
      stage === "import" ||
      parsed.retryable ||
      ["NETWORK_ERROR", "JOB_SYNC_FAILED"].includes(parsed.code);
    const paused = !retryable || delay === undefined;
    retries.set(key, {
      attempt,
      after: paused ? Infinity : Date.now() + delay!,
    });
    host.change((p) => {
      const task = Object.values(p.production?.drafts ?? {}).find(
        (t) => t.jobId === job.id || t.submissionId === job.id,
      );
      if (!task) return p;
      return saveTask(p, {
        ...task,
        status: stage === "import" ? "RECEIVING" : task.status,
        error:
          stage === "import"
            ? JSON.stringify(
                issue("RESULT_IMPORT_FAILED", parsed.details, {
                  stage: "import",
                }),
              )
            : errorText(e, "JOB_SYNC_FAILED"),
        trackingPaused: paused,
      });
    }, false);
    notify();
  }
  function receive(job: Job) {
    if ((retries.get(`import:${job.id}`)?.after ?? 0) > Date.now()) return;
    imports.add(job.id, async () => {
      if (stopped) return;
      try {
        const task = Object.values(host.get().production?.drafts ?? {}).find(
          (t) => t.jobId === job.id,
        );
        const received = new Set(
          task?.resultAssetIds ??
            (task?.resultAssetId ? [task.resultAssetId] : []),
        );
        for (let index = 0; index < terminalOutputCount(job); index++) {
          const cachedId =
            job.assets?.[index]?.id ??
            (index === 0 ? job.asset?.id : undefined);
          if (job.backgroundManaged && (!cachedId || received.has(cachedId)))
            continue;
          const asset = await host.execute<Asset>("import_job_result", {
            id: job.id,
            index,
          });
          if (stopped) return;
          // Cached imports and receiveGeneratedResult are idempotent, including
          // recovery after a partial multi-output import or application exit.
          host.change((p) => receiveGeneratedResult(p, asset, job.shot), false);
          await host.flush();
        }
        if (stopped) return;
        const complete =
          !job.backgroundManaged ||
          Array.from(
            { length: terminalOutputCount(job) },
            (_, i) =>
              job.assets?.[i]?.id ?? (i === 0 ? job.asset?.id : undefined),
          ).every(Boolean);
        host.change(
          (p) =>
            jobStatus(
              p,
              job.id,
              complete ? job.status : "RECEIVING",
              job.sync?.error ?? job.error,
              job.progress,
            ),
          false,
        );
        await host.flush();
        retries.delete(`import:${job.id}`);
        notify();
      } catch (e) {
        failed(job, e, "import");
      }
    });
  }
  function track(initial: Job) {
    if (initial.backgroundManaged) {
      // The desktop worker continues polling/downloading across project switches.
      // Frontend slots only attach already-cached outputs to the open document.
      if (!retries.has(`import:${initial.id}`)) {
        host.change((p) => {
          const next = jobStatus(
            p,
            initial.id,
            terminalOutputCount(initial) > 0 ? "RECEIVING" : initial.status,
            initial.sync?.error ?? initial.error,
            initial.progress,
            initial.requestId,
          );
          const task = Object.values(next.production?.drafts ?? {}).find(
            (t) => t.jobId === initial.id,
          );
          return task && !!task.trackingPaused !== !!initial.sync?.paused
            ? saveTask(next, {
                ...task,
                trackingPaused: !!initial.sync?.paused,
              })
            : next;
        }, false);
      }
      const count = terminalOutputCount(initial);
      const cached = Array.from(
        { length: count },
        (_, i) =>
          initial.assets?.[i]?.id ?? (i === 0 ? initial.asset?.id : undefined),
      );
      const task = Object.values(host.get().production?.drafts ?? {}).find(
        (t) => t.jobId === initial.id,
      );
      const received = new Set(
        task?.resultAssetIds ??
          (task?.resultAssetId ? [task.resultAssetId] : []),
      );
      if (
        count > 0 &&
        (cached.every(Boolean) || cached.some((id) => id && !received.has(id)))
      )
        receive(initial);
      return;
    }
    if (terminalOutputCount(initial) > 0) {
      receive(initial);
      return;
    }
    if ((retries.get(`status:${initial.id}`)?.after ?? 0) > Date.now()) return;
    queries.add(initial.id, async () => {
      if (stopped) return;
      let job = initial;
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
          job = await host.execute<Job>("refresh_job", { id: job.id });
        if (stopped) return;
        host.change(
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
        retries.delete(`status:${job.id}`);
        if (terminalOutputCount(job) > 0) receive(job);
        notify();
      } catch (e) {
        failed(job, e, "status");
      }
    });
  }
  async function poll() {
    if (stopped || listing) return;
    listing = true;
    try {
      const jobs = await host.execute<Job[]>("list_jobs", { projectId });
      if (stopped) return;
      const project = host.get();
      const paused = new Set(
        Object.values(project.production?.drafts ?? {})
          .filter((t) => t.trackingPaused)
          .map((t) => t.jobId ?? t.submissionId),
      );
      for (const job of jobs) {
        // Pausing provider queries must not hide state already confirmed by
        // the desktop worker. Managed jobs only reconcile local snapshots here.
        if (
          (job.backgroundManaged || !paused.has(job.id)) &&
          needsTracking(job, project)
        )
          track(job);
      }
    } catch (e) {
      if (!stopped) host.error(errorText(e, "JOB_SYNC_FAILED"));
    } finally {
      listing = false;
    }
  }
  return {
    poll,
    retry(id: string) {
      retries.delete(`status:${id}`);
      retries.delete(`import:${id}`);
      return poll();
    },
    stop() {
      stopped = true;
      queries.close();
      imports.close();
    },
  };
}
