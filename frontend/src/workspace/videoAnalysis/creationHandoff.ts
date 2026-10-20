import type { Project, ProjectSummary } from "../../model";
import type { CreativeTask } from "../../creative/aiTasks";
import type { AnalysisInvoke, AnalysisState } from "./types";
import {
  analysisCreationTask,
  analysisProject,
  analysisProjectKey,
} from "./creationBrief";
import { t } from "../../i18n";
export type CreationHandoff = { project: Project; task: CreativeTask };
type DraftStorage = Pick<Storage, "getItem" | "setItem" | "removeItem">;

export function createAnalysisLauncher(
  invoke: AnalysisInvoke,
  storage: DraftStorage,
) {
  const pending = new Map<string, Promise<CreationHandoff>>();
  async function open(
    state: AnalysisState,
    key: string,
  ): Promise<CreationHandoff> {
    const existing = storage.getItem(key);
    if (existing) {
      const projects = await invoke<ProjectSummary[]>("list_projects");
      if (projects.some((project) => project.id === existing)) {
        const project = await invoke<Project>("get_project", {
          projectId: existing,
        });
        // An empty suggestion focuses the composer without replacing the user's saved draft.
        return {
          project,
          task: { text: "", agentId: "coordinator", refs: [], isolated: false },
        };
      }
    }
    const task = analysisCreationTask(state);
    const project = analysisProject(state);
    const draftKey = `mstudio-chat-draft:${project.id}`;
    // Reserve the identity and draft before creating; an interrupted response can be recovered.
    try {
      storage.setItem(draftKey, task.text);
      storage.setItem(key, project.id);
    } catch (error) {
      storage.removeItem(draftKey);
      throw error;
    }
    try {
      await invoke("create_project", { document: project });
    } catch (error) {
      try {
        const projects = await invoke<ProjectSummary[]>("list_projects");
        if (projects.some((item) => item.id === project.id)) {
          return {
            project: await invoke<Project>("get_project", {
              projectId: project.id,
            }),
            task,
          };
        }
        if (storage.getItem(key) === project.id) storage.removeItem(key);
        storage.removeItem(draftKey);
      } catch {
        // Keep the reservation when the outcome is unknown. A retry checks it first.
      }
      throw error;
    }
    return { project, task };
  }
  return {
    hasProject(state: AnalysisState) {
      try {
        return !!state.result && !!storage.getItem(analysisProjectKey(state));
      } catch {
        return false;
      }
    },
    create(state: AnalysisState) {
      if (!state.result || state.busy || state.openingRecord)
        return Promise.reject(new Error(t("请先完成视频拆解")));
      const key = analysisProjectKey(state);
      const running = pending.get(key);
      if (running) return running;
      const task = open(structuredClone(state), key).finally(() =>
        pending.delete(key),
      );
      pending.set(key, task);
      return task;
    },
  };
}
