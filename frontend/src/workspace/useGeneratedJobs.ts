import { useEffect, useRef } from "react";
import { native } from "../bridge";
import { runtime } from "../plugins/runtime";
import { generatedJobTracker } from "./generatedJobTracker";
import type { Project } from "../model";
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
    const tracker = generatedJobTracker(projectId, {
      get: () => latest.current.get(),
      change: (fn, record) => latest.current.change(fn, record),
      flush: () => latest.current.flush(),
      error: (message) => latest.current.error(message),
      notify: () => window.dispatchEvent(new Event("studio-job-updated")),
      execute: (command, args) => runtime.execute(command, args),
    });
    const submitted = (event: Event) => {
      const detail = (event as CustomEvent<{ projectId: string; id: string }>)
        .detail;
      if (detail.projectId === projectId && detail.id)
        void tracker.retry(detail.id);
    };
    window.addEventListener("studio-job-submitted", submitted);
    const timer = window.setInterval(() => void tracker.poll(), 4000);
    void tracker.poll();
    return () => {
      tracker.stop();
      clearInterval(timer);
      window.removeEventListener("studio-job-submitted", submitted);
    };
  }, [projectId]);
}
