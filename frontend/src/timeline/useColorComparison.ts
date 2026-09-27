import { useEffect, useMemo, useState } from "react";
import type { Project } from "../model";
export function useColorComparison(project: Project) {
  const [id, setId] = useState<string | null>(null);
  useEffect(() => {
    const receive = (event: Event) =>
      setId((event as CustomEvent<string | null>).detail);
    window.addEventListener("studio-color-compare", receive);
    return () => window.removeEventListener("studio-color-compare", receive);
  }, []);
  return useMemo(
    () =>
      id
        ? {
            ...project,
            clips: project.clips.map((c) =>
              c.id === id ? { ...c, visual: undefined } : c,
            ),
          }
        : project,
    [id, project],
  );
}
