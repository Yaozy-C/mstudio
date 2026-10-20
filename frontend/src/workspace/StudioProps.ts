import type { Project } from "../model";
import type { CreativeTask } from "../creative/aiTasks";

export type StudioProps = {
  initial: Project;
  initialTask?: CreativeTask;
  onBack: () => void;
};
