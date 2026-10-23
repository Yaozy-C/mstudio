import type { Project, ProjectSummary } from "../../model";
import type { CreativeTask } from "../../creative/aiTasks";
import type { AnalysisInvoke, AnalysisState } from "./types";
import { analysisCreationTask, analysisProject } from "./creationBrief";
import { t } from "../../i18n";
export type CreationHandoff = { project: Project; task: CreativeTask };
type DraftStorage = Pick<Storage, "setItem" | "removeItem">;

export function createAnalysisLauncher(
  invoke: AnalysisInvoke,
  storage: DraftStorage,
) {
  // Coalesce a double click only while that action is in flight. The analysis
  // remains independent; every subsequent creation gets its own project ID.
  const pending = new WeakMap<
    NonNullable<AnalysisState["result"]>,
    Promise<CreationHandoff>
  >();
  async function create(state: AnalysisState): Promise<CreationHandoff> {
    const task = analysisCreationTask(state);
    const project = analysisProject(state);
    const draftKey = `mstudio-chat-draft:${project.id}`;
    storage.setItem(draftKey, task.text);
    try {
      await invoke("create_project", { document: project });
    } catch (error) {
      // Resolve a lost response against this action's exact identity, without
      // binding this analysis to that project or resubmitting the creation.
      const projects = await invoke<ProjectSummary[]>("list_projects").catch(
        (lookup) => {
          throw new Error(`${String(error)}; ${String(lookup)}`);
        },
      );
      if (!projects.some((item) => item.id === project.id)) {
        storage.removeItem(draftKey);
        throw error;
      }
      return {
        project: await invoke<Project>("get_project", {
          projectId: project.id,
        }),
        task,
      };
    }
    return { project, task };
  }
  return {
    create(state: AnalysisState) {
      if (!state.result || state.busy || state.openingRecord)
        return Promise.reject(new Error(t("请先完成视频拆解")));
      const result = state.result;
      const running = pending.get(result);
      if (running) return running;
      const action = create(structuredClone(state)).finally(() =>
        pending.delete(result),
      );
      pending.set(result, action);
      return action;
    },
  };
}
