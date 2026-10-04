import type { GenerationPreferences, ProductionTask } from "./types";

export type ProductionTurn = {
  projectId: string;
  task?: ProductionTask;
  referencedTask?: ProductionTask;
  models: GenerationPreferences;
  instruction: string;
};
