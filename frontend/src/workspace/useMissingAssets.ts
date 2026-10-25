import { referenceAssets } from "../previs/media";
import { useEffect, useRef } from "react";
import { bridge, native } from "../bridge";
import type { Project } from "../model";

export function applyFileStatus(
  project: Project,
  status: Record<string, boolean>,
): Project {
  if (
    !referenceAssets(project).some(
      (a) => a.id in status && !!a.missing !== status[a.id],
    )
  )
    return project;
  return {
    ...project,
    assets: project.assets.map((a) =>
      a.id in status ? { ...a, missing: status[a.id] } : a,
    ),
    ...(project.previsAssets
      ? {
          previsAssets: project.previsAssets.map((a) =>
            a.id in status ? { ...a, missing: status[a.id] } : a,
          ),
        }
      : {}),
  };
}

export function useMissingAssets(
  project: Project,
  change: (fn: (p: Project) => Project, record?: boolean) => void,
) {
  const latest = useRef(referenceAssets(project));
  latest.current = referenceAssets(project);
  useEffect(() => {
    if (!native) return;
    let stopped = false,
      checking = false;
    const check = async () => {
      if (checking || stopped) return;
      checking = true;
      try {
        const requested = latest.current;
        const status = await bridge<Record<string, boolean>>(
          "asset_file_status",
          { projectId: project.id, ids: requested.map((a) => a.id) },
        );
        const current = Object.fromEntries(
          Object.entries(status).filter(
            ([id]) =>
              latest.current.find((a) => a.id === id)?.path ===
              requested.find((a) => a.id === id)?.path,
          ),
        );
        if (!stopped) change((p) => applyFileStatus(p, current), false);
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
