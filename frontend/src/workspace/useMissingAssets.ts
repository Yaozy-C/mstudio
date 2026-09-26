import { useEffect, useRef } from "react";
import { bridge, native } from "../bridge";
import type { Project } from "../model";

export function applyFileStatus(
  project: Project,
  status: Record<string, boolean>,
): Project {
  if (
    !project.assets.some((a) => a.id in status && !!a.missing !== status[a.id])
  )
    return project;
  return {
    ...project,
    assets: project.assets.map((a) =>
      a.id in status ? { ...a, missing: status[a.id] } : a,
    ),
  };
}

export function useMissingAssets(
  project: Project,
  change: (fn: (p: Project) => Project, record?: boolean) => void,
) {
  const latest = useRef(project.assets);
  latest.current = project.assets;
  useEffect(() => {
    if (!native) return;
    let stopped = false,
      checking = false;
    const check = async () => {
      if (checking || stopped) return;
      checking = true;
      try {
        const status = await bridge<Record<string, boolean>>(
          "asset_file_status",
          { ids: latest.current.map((a) => a.id) },
        );
        if (!stopped) change((p) => applyFileStatus(p, status), false);
      } catch {
        /* Keep the last known state when the bridge is unavailable. */
      } finally {
        checking = false;
      }
    };
    void check();
    const interval = window.setInterval(() => void check(), 5000);
    window.addEventListener("focus", check);
    return () => {
      stopped = true;
      clearInterval(interval);
      window.removeEventListener("focus", check);
    };
  }, [project.id, change]);
}
