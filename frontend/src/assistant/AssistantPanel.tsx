import { ErrorNotice } from "../errors/ErrorNotice";
import { MediaComposer } from "./MediaComposer";
import { ConversationTimeline } from "./ConversationTimeline";
import { defaultAgent } from "./workContext";
import type { WorkContext } from "./workContext";
import { listen } from "@tauri-apps/api/event";
import type { AgentProgress } from "./streamReply";
import { followGeneration } from "../production/followGeneration";
import { useComposerDraft } from "./useComposerDraft";
import { createChatAdapter, type MessageTarget } from "./chatAdapter";
import { taskAttachments } from "../production/chat";
import type { ProductionController } from "../production/useProduction";
import { useEffect, useRef, useState } from "react";
import {
  AssistantRuntimeProvider,
  ThreadPrimitive,
  useLocalRuntime,
  type ThreadMessageLike,
} from "@assistant-ui/react";
import { bridge, native } from "../bridge";
import type { Project } from "../model";
import type { PlaybackClock } from "../timeline/clock";
import { useAgentHistory } from "./useAgentHistory";
import { AgentWelcome } from "./AgentWelcome";
import "../styles/agent-welcome.css";
import { MessageContext } from "./AgentMessage";
import { AgentComposer } from "./AgentComposer";
import { useAgents } from "../agents/catalog";
import { useAgentModel } from "../models/useAgentModel";
import type { Attachment } from "./attachments";
import type { AttachmentDraft } from "./useAttachments";
type Props = {
  visible?: boolean;
  canvas?: ProductionController;
  project: Project;
  nodeId: string | null;
  clipId: string | null;
  clock: PlaybackClock;
  draft: AttachmentDraft;
  flush: () => Promise<void>;
  work?: WorkContext;
  onSettings: () => void;
};
export function AssistantPanel(props: Props) {
  return <AgentPanel {...props} />;
}
function AgentPanel(props: Props) {
  const { history, error } = useAgentHistory(props.project.id);
  if (!history)
    return <div className="agent-empty">{error || "加载对话…"}</div>;
  return <Chat {...props} history={history} />;
}
function Chat(props: Props & { history: ThreadMessageLike[] }) {
  const [newTask, setNewTask] = useState(false);
  const latest = useRef({ ...props, newTask });
  latest.current = { ...props, newTask };
  const input = useRef<HTMLTextAreaElement>(null);
  const [sent, setSent] = useState<Record<string, Attachment[]>>({});
  const [error, setError] = useState("");
  const agents = useAgents();
  const [agentId, setAgentId] = useState<string | null>(null);
  const effectiveAgent = agentId ?? defaultAgent(props.work);
  const chosen = agents.agents.find((a) => a.id === effectiveAgent);
  const agentRef = useRef(agentId);
  agentRef.current = agentId;
  const [targets, setTargets] = useState<Record<string, MessageTarget>>({});
  const models = useAgentModel(props.project.id);
  const routing = useRef({ agents: agents.agents, catalog: models.catalog });
  routing.current = { agents: agents.agents, catalog: models.catalog };
  const agentReady = !!chosen?.enabled;
  useEffect(() => {
    if (!props.draft.focus) return;
    const frame = requestAnimationFrame(() => input.current?.focus());
    return () => cancelAnimationFrame(frame);
  }, [props.draft.focus]);
  useEffect(
    () => () => {
      if (native)
        void bridge("cancel_assistant", { threadId: props.project.id });
    },
    [props.project.id],
  );
  const [adapter] = useState(() =>
    createChatAdapter({
      context: () => latest.current,
      routing: () => routing.current,
      agent: () => agentRef.current,
      accepted: (id, refs) => {
        setSent((current) => ({ ...current, [id]: refs }));
        setNewTask(false);
      },
      target: (id, target) =>
        setTargets((current) => ({ ...current, [id]: target })),
      running: models.setRunning,
      error: setError,
      invoke: bridge,
      subscribe: native
        ? (receive) =>
            listen<AgentProgress>("agent-progress", ({ payload }) =>
              receive(payload),
            )
        : undefined,
    }),
  );
  const runtime = useLocalRuntime(adapter, { initialMessages: props.history });
  useComposerDraft(props.project.id, runtime);
  useEffect(() => {
    if (!props.draft.suggestion) return;
    props.canvas?.setComposerMode("agent");
    setAgentId(props.draft.suggestion.agentId ?? "production");
    const previous = runtime.thread.composer.getState().text;
    if (props.draft.suggestion.text.trim())
      runtime.thread.composer.setText(
        [previous, props.draft.suggestion.text].filter(Boolean).join("\n\n"),
      );
    props.draft.clearSuggestion();
    input.current?.focus();
  }, [props.draft.suggestion, runtime]);
  // Keep the agent runtime alive while its expensive message tree is hidden.
  if (props.visible === false) return null;
  const attachments = taskAttachments(
    props.project,
    props.canvas?.task,
    props.draft.items,
  );
  return (
    <MessageContext.Provider
      value={{
        project: props.project,
        sent,
        targets,

        canvas: props.canvas,
        settings: props.onSettings,
        follow: (task, text, kind) => {
          const next = followGeneration(latest.current.canvas, task, kind);
          if (text && next)
            latest.current.canvas?.update({ prompt: text }, next.key);
          input.current?.focus();
        },
      }}
    >
      <AssistantRuntimeProvider runtime={runtime}>
        <ThreadPrimitive.Root className="agent-thread">
          <ThreadPrimitive.Viewport className="agent-messages">
            {!props.canvas?.runs.length && (
              <ThreadPrimitive.Empty>
                <AgentWelcome
                  hasContent={props.project.nodes.length > 0}
                  editingFrame={false}
                  connected={models.available && agentReady}
                  connect={props.onSettings}
                  suggest={(text, id) => {
                    setAgentId(id);
                    runtime.thread.composer.setText(text);
                    input.current?.focus();
                  }}
                />
              </ThreadPrimitive.Empty>
            )}
            <ConversationTimeline
              canvas={props.canvas}
              project={props.project}
              settings={props.onSettings}
            />
          </ThreadPrimitive.Viewport>
          {(error || models.error || agents.error) && (
            <ErrorNotice
              error={error || models.error || agents.error}
              fallback="OPERATION_FAILED"
            />
          )}
          {props.canvas?.task?.error && (
            <ErrorNotice
              error={props.canvas.task.error}
              fallback="VALIDATION_FAILED"
            />
          )}
          {props.draft.notice && (
            <p className="agent-attachment-hint" role="status">
              {props.draft.notice}
            </p>
          )}
          {props.canvas && props.canvas.composerMode !== "agent" ? (
            <MediaComposer
              canvas={props.canvas}
              project={props.project}
              draft={props.draft}
              settings={props.onSettings}
            />
          ) : (
            <AgentComposer
              newTask={newTask}
              toggleNewTask={() => setNewTask((value) => !value)}
              canvas={props.canvas}
              work={props.work}
              input={input}
              project={props.project}
              draft={props.draft}
              attachments={attachments}
              agents={agents.agents}
              agentId={agentId}
              chooseAgent={setAgentId}
              getText={() => runtime.thread.composer.getState().text}
              setText={(text) => {
                runtime.thread.composer.setText(text);
              }}
              running={models.running}
              ready={models.available && agentReady}
              model={models.selected}
              settings={props.onSettings}
            />
          )}
        </ThreadPrimitive.Root>
      </AssistantRuntimeProvider>
    </MessageContext.Provider>
  );
}
