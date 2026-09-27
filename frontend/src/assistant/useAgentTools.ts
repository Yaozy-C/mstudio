import { savedValues, clipReceipt } from "./savedValues";
import { scriptReceipt } from "./scriptReceipt";
import { productionTurn, type ProductionTurn } from "../production/turnContext";
import { runsOf } from "../production/requestTask";
import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { bridge, native } from "../bridge";
import { applyOperations, inspectProject } from "./projectCommands";
import { flushPendingEdits } from "../workspace/pendingEdits";
import type { useProject } from "../workspace/useProject";
type Request = {
  callId: string;
  projectId: string;
  agentId: string;
  turnId: string;
  generation?: { turnId: string; turn: ProductionTurn } | null;
  args: {
    action: string;
    revision?: number;
    operations?: unknown;
    nodeIds?: string[];
    offset?: number;
  };
};
export function useAgentTools(model: ReturnType<typeof useProject>) {
  const latest = useRef(model);
  latest.current = model;
  useEffect(() => {
    if (!native) return;
    let queue = Promise.resolve();
    const off = listen<Request>("agent-project-tool", ({ payload }) => {
      if (payload.projectId !== latest.current.get().id) return;
      queue = queue
        .catch(() => {})
        .then(async () => {
          const m = latest.current;
          let result: unknown;
          try {
            if (
              !(await bridge<boolean>("agent_tool_active", {
                callId: payload.callId,
                projectId: payload.projectId,
              }))
            )
              throw new Error("ABORTED_BEFORE_DISPATCH: 操作已取消，未执行");
            if (payload.projectId !== m.get().id)
              throw new Error("ABORTED_BEFORE_DISPATCH: 工程已切换，未执行");
            flushPendingEdits();
            if (payload.args.action === "inspect")
              result = inspectProject(m.get(), payload.args);
            else if (payload.args.action === "edit") {
              const before = m.get();
              const generationTurnId =
                payload.generation?.turnId ?? payload.turnId;
              const turn =
                payload.generation?.turn ?? productionTurn(payload.turnId);
              // One pure command batch and one undo entry; failed validation changes nothing.
              m.change((p) =>
                applyOperations(
                  p,
                  payload.args.revision ?? -1,
                  payload.args.operations,
                  turn
                    ? { turn, turnId: generationTurnId, callId: payload.callId }
                    : undefined,
                ),
              );
              await m.flush();
              const ops = payload.args.operations as {
                op: string;
                id?: string;
                taskKey?: string;
              }[];
              if (
                ops.some(
                  (o) =>
                    o.op === "add_node" &&
                    m
                      .get()
                      .nodes.some((n) => n.id === o.id && n.kind === "plan"),
                )
              )
                window.dispatchEvent(new Event("studio-focus-plan"));
              if (ops.some((o) => o.op === "assemble_plan"))
                window.dispatchEvent(new Event("studio-plan-assembled"));
              result = {
                applied: true,
                savedValues: savedValues(m.get(), payload.args.operations),
                savedClips: clipReceipt(before, m.get()),
                scripts: scriptReceipt(m.get(), payload.args.operations),
                updatedTasks: ops
                  .filter((o) => o.op === "update_generation")
                  .map((o) => {
                    const task = m.get().production?.drafts?.[o.taskKey ?? ""];
                    return {
                      id: task?.key,
                      status: task?.status,
                      savedForNextGeneration: task?.nextPrompt !== undefined,
                    };
                  }),
                generationTasks: runsOf(m.get(), generationTurnId)
                  .filter(
                    (t) =>
                      t.key.startsWith(`run:${payload.callId}:`) ||
                      t.key.startsWith(`retry:${payload.callId}:`),
                  )
                  .map((t) => ({
                    id: t.key,
                    targetId: ops[Number(t.key.split(":").at(-1))]?.id,
                    kind: t.kind,
                    status: t.status,
                    modelId: t.modelId,
                    message:
                      "生成操作已显示在本轮对话中；以实际任务状态为准，不要声称已生成完成",
                  })),
                revision: m.get().revision,
                message: "修改已应用并保存，可撤销",
                changed: ops.map((o) => ({
                  op: o.op,
                  id: "id" in o ? o.id : undefined,
                  fields: Object.keys(o).filter(
                    (key) => key !== "op" && key !== "id",
                  ),
                })),
              };
            } else throw new Error("未知工具操作");
          } catch (e) {
            result = { error: String(e) };
          }
          await bridge("agent_tool_result", {
            callId: payload.callId,
            projectId: payload.projectId,
            result,
          });
        })
        .catch(() => {});
    });
    return () => {
      void off.then((f) => f());
    };
  }, [model.project.id]);
}
