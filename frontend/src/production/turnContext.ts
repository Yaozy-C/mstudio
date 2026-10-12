import type { GenerationPreferences, ProductionTask } from "./types";

export type ProductionTurn = {
  projectId: string;
  canvas?: {
    view: import("../model").Project["viewport"];
    size?: { width: number; height: number };
  };
  task?: ProductionTask;
  referencedTask?: ProductionTask;
  models: GenerationPreferences;
  instruction: string;
};
