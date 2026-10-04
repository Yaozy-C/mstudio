import { DomainError } from "./domainError";
import { validateShotOrder } from "../creative/operations";
import { applyOperations } from "../assistant/projectCommands";
import { inspectProject } from "../assistant/inspectProject";
import { modelToolError } from "../assistant/toolMessages";
import { materializeFrameCards } from "../production/frameCards";
import { contentChanged } from "../workspace/projectRevision";
import { parameterContract, taskParameters } from "../production/parameters";
import { equal, mergeValue } from "./merge";
import { receipt } from "./receipt";
import { prepareGeneration, reconcileJob } from "./generation";
import { generationOutcomes } from "../production/taskOutcome";
import type { Project } from "../model";
import type { GenerationCommandContext } from "../production/requestTask";
import type { MediaModel } from "../models/mediaRegistry";
type Request = {
  action: string;
  document: Project;
  base: Project;
  current: Project;
  args: Record<string, unknown>;
  executionId: string;
  context?: GenerationCommandContext;
  models: MediaModel[];
  model: MediaModel;
  taskKey: string;
  taskKeys: string[];
  uploaded?: import("../creation/generationInput").UploadedReference[];
  job: import("../workspace/generatedJobTracking").GeneratedJob;
};
function run(input: Request) {
  if (input.action === "generation_outcomes")
    return {
      generationTasks: generationOutcomes(input.document, input.taskKeys),
    };
  if (input.action === "capabilities") return parameterContract(input.model);
  if (input.action === "prepare_generation")
    return prepareGeneration(
      input.document,
      input.taskKey,
      input.model,
      input.uploaded,
    );
  if (input.action === "job") {
    const document = reconcileJob(input.document, input.job);
    document.revision =
      (input.document.revision ?? 0) +
      Number(contentChanged(input.document, document));
    return { document };
  }
  if (input.action === "inspect")
    return inspectProject(input.document, input.args);
  if (input.action === "merge") {
    const document = mergeValue(
      input.base,
      input.document,
      input.current,
    ) as Project;
    validateShotOrder(document);
    document.revision =
      (input.current.revision ?? 0) +
      Number(contentChanged(input.current, document));
    return { document };
  }
  if (input.action !== "edit") throw new Error("Unknown domain action");
  const before = input.document;
  const operations = input.args.operations as Record<string, unknown>[];
  const document = materializeFrameCards(
    applyOperations(before, operations, input.context),
  );
  const tasks = document.production?.drafts ?? {};
  for (const [key, task] of Object.entries(tasks)) {
    if (equal(task, before.production?.drafts?.[key])) continue;
    if (!task.modelId) continue;
    const model = input.models.find((m) => m.id === task.modelId && m.enabled);
    if (!model || model.kind !== task.kind)
      throw new Error(
        "Selected generation model missing, disabled or wrong kind",
      );
    taskParameters(model, task.parameters);
  }
  document.revision =
    (before.revision ?? 0) + Number(contentChanged(before, document));
  return {
    document,
    receipt: receipt(before, document, operations, input.executionId),
  };
}
// Only JSON crosses the native boundary. The runtime exposes no filesystem,
// network, DOM or module loader; transactions and permissions belong to Rust.
Object.assign(globalThis, {
  domainExecute(raw: string): string {
    try {
      return JSON.stringify(run(JSON.parse(raw)));
    } catch (error) {
      return JSON.stringify({
        error: modelToolError(error),
        code: error instanceof DomainError ? error.code : "DOMAIN_VALIDATION",
        ...(error instanceof DomainError ? { context: error.context } : {}),
        stage: "validation",
        outcome: "not_executed",
        recovery: { action: "correct_arguments" },
      });
    }
  },
});
