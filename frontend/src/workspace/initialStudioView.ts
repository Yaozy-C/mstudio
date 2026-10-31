import type { CreativeTask } from "../creative/aiTasks";
import type { Project } from "../model";
import type { StudioView } from "./StudioStage";

export function initialStudioView(
  project: Project,
  task?: CreativeTask,
): StudioView {
  if (task?.agentId === "concept") return "concepts";
  if (task && task.agentId !== "coordinator") return "script";
  return project.nodes.length || project.clips.length ? "script" : "concepts";
}
