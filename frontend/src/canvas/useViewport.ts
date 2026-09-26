import { useCallback, useEffect, useRef, useState } from "react";
import type { Project } from "../model";
import { registerPendingEdit } from "../workspace/pendingEdits";
export function useViewport(
  initial: Project["viewport"],
  onChange: (fn: (p: Project) => Project) => void,
) {
  const [view, setView] = useState(initial);
  const latest = useRef(initial);
  const dirty = useRef(false);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const frame = useRef(0);
  const flush = useCallback(() => {
    clearTimeout(timer.current);
    if (dirty.current) {
      dirty.current = false;
      const viewport = latest.current;
      onChange((p) => ({ ...p, viewport }));
    }
  }, [onChange]);
  useEffect(() => {
    latest.current = initial;
    setView(initial);
  }, [initial]);
  useEffect(() => {
    const off = registerPendingEdit(flush);
    return () => {
      flush();
      cancelAnimationFrame(frame.current);
      off();
    };
  }, [flush]);
  const update = useCallback(
    (next: Project["viewport"]) => {
      latest.current = next;
      dirty.current = true;
      cancelAnimationFrame(frame.current);
      frame.current = requestAnimationFrame(() => setView(latest.current));
      clearTimeout(timer.current);
      timer.current = setTimeout(flush, 180);
    },
    [flush],
  );
  return { view, latest, update, flush };
}
