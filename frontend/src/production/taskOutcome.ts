import type { Project } from "../model";
import type { ProductionTask } from "./types";

type Continuation = {
  state: "waiting_user" | "waiting_service" | "ready" | "finished";
  action: string;
  trigger: string;
};
function continuation(task: ProductionTask, hasResults: boolean): Continuation {
  if (task.trackingPaused)
    return {
      state: "waiting_user",
      action: "resume_tracking",
      trigger: "user_action",
    };
  switch (task.status ?? "DRAFT") {
    case "DRAFT":
    case "AWAITING_CONFIRMATION":
      return {
        state: "waiting_user",
        action: "confirm_generation",
        trigger: "user_confirmation",
      };
    case "UNKNOWN":
      return {
        state: "waiting_user",
        action: "resolve_unknown_submission",
        trigger: "user_action",
      };
    case "FAILED":
    case "CANCELLED":
      return {
        state: "finished",
        action: "report_outcome",
        trigger: "new_user_request",
      };
    case "COMPLETED":
      if (hasResults)
        return {
          state: "ready",
          action: "inspect_result",
          trigger: "result_available",
        };
      return {
        state: "waiting_service",
        action: "await_generation",
        trigger: "generation_event",
      };
    case "READY":
    case "UPLOADING":
    case "SUBMITTING":
    case "IN_QUEUE":
    case "IN_PROGRESS":
    case "RECEIVING":
    case "CANCEL_REQUESTED":
      return {
        state: "waiting_service",
        action: "await_generation",
        trigger: "generation_event",
      };
    default:
      return {
        state: "waiting_user",
        action: "resolve_task_state",
        trigger: "user_action",
      };
  }
}

export function taskOutcome(task: ProductionTask) {
  const resultAssetIds = [
    ...new Set([
      ...(task.resultAssetIds ?? []),
      ...(task.resultAssetId ? [task.resultAssetId] : []),
    ]),
  ];
  return {
    id: task.key,
    targetId: task.targetNodeId ?? task.ownerId,
    kind: task.kind,
    modelId: task.modelId,
    parameters: task.parameters,
    status: task.status ?? "DRAFT",
    resultAssetIds,
    error: task.error,
    continuation: continuation(task, resultAssetIds.length > 0),
  };
}

export function generationOutcomes(p: Project, keys: string[]) {
  return keys.map((id) => {
    const task = p.production?.drafts?.[id];
    return task
      ? taskOutcome(task)
      : {
          id,
          status: "REMOVED",
          resultAssetIds: [],
          continuation: {
            state: "finished",
            action: "report_missing_task",
            trigger: "new_user_request",
          },
        };
  });
}
