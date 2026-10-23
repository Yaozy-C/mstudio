import type { Project } from "../model";
import type { GeneratedJob } from "../workspace/generatedJobTracking";
import { saveTask } from "../production/document";

export const jobIssue = (job: GeneratedJob) =>
  job.reconciliation?.error ?? job.sync?.error ?? job.error;

// Tracking updates never receive output assets or change the provider's status.
// They must still be visible when result reconciliation itself is failing.
export function syncJobTracking(project: Project, job: GeneratedJob): Project {
  const task = Object.values(project.production?.drafts ?? {}).find(
    (task) => task.jobId === job.id || task.submissionId === job.id,
  );
  if (!task) return project;
  const error = jobIssue(job);
  const trackingPaused = !!job.sync?.paused;
  if (task.error === error && !!task.trackingPaused === trackingPaused)
    return project;
  return saveTask(project, { ...task, error, trackingPaused });
}
