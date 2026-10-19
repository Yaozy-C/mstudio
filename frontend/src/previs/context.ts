import { createContext, useContext } from "react";
import type { Project } from "../model";
import type { PrevisFile, PrevisWorkspace } from "./types";
import type { PrevisTaskSummary } from "./PrevisTaskList";
export const openPrevis = (fileId: string) =>
  window.dispatchEvent(
    new CustomEvent("mstudio-open-previs", { detail: fileId }),
  );
type PrevisContext = {
  workspace: PrevisWorkspace;
  project: Project;
  change: (fn: (p: Project) => Project) => void;
  refresh: () => Promise<void>;
  save: (file: PrevisFile) => Promise<PrevisFile>;
  flush: () => Promise<unknown>;
  open: (id: string) => void;
  createModel: () => void;
  error: string;
  loading: boolean;
  tasks: PrevisTaskSummary[];
  videoPanel: { open: boolean; fileId?: string; autoPlay?: boolean };
  showVideos: (fileId?: string, autoPlay?: boolean) => void;
  closeVideos: () => void;
  attachReference: (
    fileId: string,
    videoId: string,
    assetId: string,
  ) => Promise<void>;
  collectVideo: (videoId: string, assetId: string) => Promise<void>;
};
export const Context = createContext<PrevisContext | null>(null);
export function usePrevis() {
  const c = useContext(Context);
  if (!c) throw Error("Missing previsualization workspace");
  return c;
}
export function useOptionalPrevis() {
  return useContext(Context);
}
