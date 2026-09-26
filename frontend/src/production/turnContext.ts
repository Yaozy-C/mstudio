import type { GenerationPreferences, ProductionTask } from "./types";

export type ProductionTurn = {
  projectId: string;
  task?: ProductionTask;
  referencedTask?: ProductionTask;
  models: GenerationPreferences;
  instruction: string;
};
const turns = new Map<string, ProductionTurn>();
export function beginProductionTurn(id: string, context: ProductionTurn) {
  turns.set(id, structuredClone(context));
}
export function productionTurn(id?: string) {
  return id ? turns.get(id) : undefined;
}
export function endProductionTurn(id: string) {
  turns.delete(id);
}
