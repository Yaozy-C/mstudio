import { useCallback, useEffect, useRef, useState } from "react";
import type { Project } from "../model";
import {
  rememberedViewport,
  rememberViewport,
  persistViewport,
} from "./viewportMemory";
export function useViewport(project: Project) {
  const [restored] = useState(
    () => rememberedViewport(project.id) ?? project.production?.viewport,
  );
  const [view, setView] = useState(restored ?? project.viewport);
  const latest = useRef(view);
  const dirty = useRef(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const frame = useRef(0);
  const flush = useCallback(() => {
    clearTimeout(timer.current);
    if (dirty.current) {
      dirty.current = false;
      persistViewport(project.id);
    }
  }, [project.id]);
  useEffect(() => {
    window.addEventListener("pagehide", flush);
    return () => {
      flush();
      cancelAnimationFrame(frame.current);
      window.removeEventListener("pagehide", flush);
    };
  }, [flush]);
  const update = useCallback(
    (next: Project["viewport"]) => {
      latest.current = next;
      rememberViewport(project.id, next);
      dirty.current = true;
      cancelAnimationFrame(frame.current);
      frame.current = requestAnimationFrame(() => setView(latest.current));
      clearTimeout(timer.current);
      timer.current = setTimeout(flush, 180);
    },
    [project.id, flush],
  );
  return { view, latest, update, hasSavedView: !!restored };
}
