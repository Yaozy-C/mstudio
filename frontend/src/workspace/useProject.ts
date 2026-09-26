import { materializeFrameCards } from "../production/frameCards";
import { flushPendingEdits } from "./pendingEdits";
import { registerExitSave } from "./useSafeExit";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";
import { saveProject } from "../bridge";
import type { Project } from "../model";
import { restoreProject } from "./restoreProject";
import { createProjectAutosave } from "./projectAutosave";
export function useProject(initial: Project) {
  const [project, setProject] = useState(() => materializeFrameCards(initial));
  const [autosave] = useState(() =>
    createProjectAutosave(initial, saveProject),
  );
  useEffect(() => {
    if (project !== initial) autosave.update(project);
  }, []);
  const saved = useSyncExternalStore(autosave.subscribe, autosave.getStatus);
  const [history, setHistory] = useState<{
    past: Project[];
    future: Project[];
  }>({ past: [], future: [] });
  const latest = useRef(project);
  latest.current = project;
  const get = useCallback(() => latest.current, []);
  const change = useCallback(
    (fn: (p: Project) => Project, record = true) => {
      const previous = latest.current;
      const value = fn(previous);
      if (value === previous) return;
      const next = {
        ...materializeFrameCards(value),
        revision: (previous.revision || 0) + 1,
      };
      latest.current = next;
      autosave.update(next);
      if (record && next.viewport === previous.viewport) {
        setHistory((h) => ({
          past: [...h.past.slice(-49), previous],
          future: [],
        }));
      }
      setProject(next);
    },
    [autosave],
  );
  const undo = () =>
    setHistory((h) => {
      if (!h.past.length) return h;
      const previous = restoreProject(h.past.at(-1)!, latest.current);
      setProject(previous);
      latest.current = previous;
      autosave.update(previous);
      return { past: h.past.slice(0, -1), future: [project, ...h.future] };
    });
  const redo = () =>
    setHistory((h) => {
      if (!h.future.length) return h;
      const next = restoreProject(h.future[0], latest.current);
      setProject(next);
      latest.current = next;
      autosave.update(next);
      return { past: [...h.past, project], future: h.future.slice(1) };
    });
  const flush = useCallback(() => {
    flushPendingEdits();
    return autosave.flush();
  }, [autosave]);
  useEffect(() => {
    const off = registerExitSave(flush);
    const saveOnPageHide = () => {
      void flush().catch(() => {});
    };
    window.addEventListener("pagehide", saveOnPageHide);
    return () => {
      off();
      window.removeEventListener("pagehide", saveOnPageHide);
      void flush().catch(() => {});
    };
  }, [autosave, flush]);
  return {
    project,
    get,
    change,
    undo,
    redo,
    canUndo: !!history.past.length,
    canRedo: !!history.future.length,
    saved,
    flush,
  };
}
