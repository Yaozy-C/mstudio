import { defaultAgent, type WorkContext } from "./workContext";
import { streamReply, type SubscribeProgress } from "./streamReply";
import type { ChatModelAdapter } from "@assistant-ui/react";
import type { AgentProfile } from "../agents/catalog";
import type { bridge } from "../bridge";
import type { Project } from "../model";
import { readyModel, type ModelCatalog } from "../models/types";
import { taskAttachments, taskContext } from "../production/chat";
import {
  beginProductionTurn,
  endProductionTurn,
  type ProductionTurn,
} from "../production/turnContext";
import type { ProductionController } from "../production/useProduction";
import type { PlaybackClock } from "../timeline/clock";
import type { Attachment } from "./attachments";
import { resolveMention } from "./mentions";
import type { AttachmentDraft } from "./useAttachments";

export type ChatContext = {
  project: Project;
  work?: WorkContext;
  canvas?: ProductionController;
  draft: AttachmentDraft;
  nodeId: string | null;
  clipId: string | null;
  clock: PlaybackClock;
  flush: () => Promise<void>;
};
export type MessageTarget = {
  agentId: string;
  agentName: string;
  targetNodeId: string | null;
};
export type SentTurn = {
  work?: WorkContext;
  refs: Attachment[];
  agentId: string;
  targetNodeId: string | null;
  selectedNodeId: string | null;
  selection: { clipId: string | null; time: number };
  production: ProductionTurn;
};
type Options = {
  context: () => ChatContext;
  routing: () => { agents: AgentProfile[]; catalog: ModelCatalog };
  agent: () => string | null;
  accepted: (id: string, refs: Attachment[]) => void;
  target: (id: string, target: MessageTarget) => void;
  running: (value: boolean) => void;
  error: (value: string) => void;
  invoke: typeof bridge;
  subscribe?: SubscribeProgress;
};

export function createChatAdapter(options: Options): ChatModelAdapter {
  const turns = new Map<string, SentTurn>();
  const attempts = new Map<string, string>();
  return {
    async *run({ messages, abortSignal }) {
      const p = options.context();
      if (p.canvas && p.canvas.composerMode !== "agent")
        throw new Error("图片和视频模式请使用生成入口");
      const work = p.work ? { view: p.work.view } : undefined;
      const message = messages.at(-1)!;
      options.running(true);
      options.error("");
      let generationTurnId: string | undefined;
      const cancel = () =>
        void options.invoke("cancel_assistant", { threadId: p.project.id });
      try {
        let turn = turns.get(message.id);
        if (!turn) {
          const meta = message.metadata.custom;
          const saved = meta?.request as SentTurn | undefined;
          if (meta?.turnId && !saved)
            throw new Error(
              "这条消息缺少重试上下文，请引用需要的素材后重新发送",
            );
          const focus = saved
            ? { target: saved.targetNodeId, agent: saved.agentId }
            : { target: p.draft.targetNodeId, agent: defaultAgent(work) };
          turn = structuredClone(
            saved ?? {
              work,
              refs:
                (meta?.attachments as Attachment[] | undefined) ??
                taskAttachments(p.project, p.canvas?.task, p.draft.items),
              agentId:
                (meta?.agentId as string | undefined) ??
                options.agent() ??
                focus.agent,
              targetNodeId:
                (meta?.taskTarget as { id?: string } | undefined)?.id ??
                focus.target,
              selectedNodeId: p.canvas?.task
                ? (p.canvas.task.ownerId ?? null)
                : null,
              selection: { clipId: null, time: 0 },
              production: {
                projectId: p.project.id,
                task: p.canvas?.task,
                referencedTask: p.canvas?.referencedTask,
                models: {
                  ...(p.canvas?.modelPreferences ??
                    p.project.production?.models ??
                    {}),
                },
                instruction: message.content
                  .filter((c) => c.type === "text")
                  .map((c) => c.text)
                  .join("\n"),
              },
            },
          );
          turns.set(message.id, turn);
        }
        // Read on every new turn and retry, including updates from another window.
        const agents = await options.invoke<AgentProfile[]>("agent_catalog");
        if (abortSignal.aborted) throw new Error("已停止");
        const routing = options.routing();
        const { agent, model: selected } = resolveMention(
          agents,
          routing.catalog,
          turn.agentId,
        );
        if (!readyModel(selected))
          throw new Error(`请检查 ${agent.name} 的模型连接`);
        // Validate routing before consuming the visible draft.
        if (!message.metadata.custom?.request && !attempts.has(message.id)) {
          options.accepted(message.id, turn.refs);
          p.draft.consume(turn.refs);
          p.draft.consumeTarget(turn.targetNodeId);
          p.canvas?.clearTaskReference?.(turn.production.referencedTask?.key);
          if (p.canvas?.task)
            p.canvas.update({ inputs: [], ownerId: undefined });
        }
        const custom = {
          createdAt: message.createdAt.getTime(),
          agentId: agent.id,
          agentName: agent.name,
          modelName: selected.name,
          turnId: crypto.randomUUID(),
        };
        options.target(message.id, {
          agentId: agent.id,
          agentName: agent.name,
          targetNodeId: turn.targetNodeId,
        });
        generationTurnId = custom.turnId;
        beginProductionTurn(custom.turnId, turn.production);
        yield { content: [], metadata: { custom } };
        await p.flush();
        if (abortSignal.aborted) throw new Error("已停止");
        abortSignal.addEventListener("abort", cancel, { once: true });
        const resumeTurnId =
          attempts.get(message.id) ??
          (message.metadata.custom?.turnId as string | undefined);
        attempts.set(message.id, custom.turnId);
        for await (const text of streamReply(
          options.invoke,
          {
            resumeTurnId,
            messageContext: turn,
            agentName: agent.name,
            modelName: selected.name,
            clientTurnId: custom.turnId,
            modelId: selected.id,
            agentId: agent.id,
            agentRevision: agent.revision,
            projectId: p.project.id,
            prompt:
              turn.production.instruction +
              taskContext(
                turn.production.task,
                turn.production.models,
                turn.production.referencedTask,
              ),
            attachments: turn.refs.map(({ kind, id }) => ({ kind, id })),
            selectedNodeId: turn.selectedNodeId,
            taskNodeId: turn.targetNodeId,
            selection: turn.selection,
          },
          abortSignal,
          options.subscribe,
        )) {
          yield { content: [{ type: "text", text }], metadata: { custom } };
        }
      } catch (e) {
        options.error(String(e));
        throw e;
      } finally {
        if (generationTurnId) endProductionTurn(generationTurnId);
        options.running(false);
        abortSignal.removeEventListener("abort", cancel);
      }
    },
  };
}
