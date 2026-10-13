import type { Project } from "../model";
import type { GeneratedJob } from "../workspace/generatedJobTracking";
import { contentChanged } from "../workspace/projectRevision";
import { reconcileJob } from "./generation";

// A bad historical job must not prevent the remaining jobs from recovering.
export function recoverJobs(document: Project, jobs: GeneratedJob[]) {
  let failedJobs = 0;
  for (const job of jobs) {
    try {
      const next = reconcileJob(document, job);
      next.revision =
        (document.revision ?? 0) + Number(contentChanged(document, next));
      document = next;
    } catch {
      failedJobs++;
    }
  }
  return { document, failedJobs };
}
