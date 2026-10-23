import type { Project } from "../model";
import type { GeneratedJob } from "../workspace/generatedJobTracking";
import { contentChanged } from "../workspace/projectRevision";
import { reconcileJob } from "./generation";
import { issue } from "../errors/catalog";
import { syncJobTracking } from "./generationTracking";

// A failed result update must not prevent the remaining jobs from syncing.
export function recoverJobs(document: Project, jobs: GeneratedJob[]) {
  const failures: { jobId: string; error: string }[] = [];
  for (const job of jobs) {
    try {
      // Stale local errors clear only on success. A failure must retain the
      // original pause/backoff metadata when reporting its diagnostic.
      const next = reconcileJob(document, {
        ...job,
        reconciliation: undefined,
        sync: job.sync?.stage === "reconcile" ? undefined : job.sync,
      });
      next.revision =
        (document.revision ?? 0) + Number(contentChanged(document, next));
      document = next;
    } catch (error) {
      const diagnostic = JSON.stringify(
        issue(
          "PROJECT_SYNC_FAILED",
          error instanceof Error ? error.message : String(error),
          { stage: "reconcile", retryable: true },
        ),
      );
      failures.push({ jobId: job.id, error: diagnostic });
      document = syncJobTracking(document, {
        ...job,
        reconciliation: { error: diagnostic },
      });
    }
  }
  return { document, failures };
}
