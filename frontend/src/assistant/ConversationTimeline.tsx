import { ThreadPrimitive, useAuiState } from "@assistant-ui/react";
import { AgentMessage } from "./AgentMessage";
import { TaskEntry } from "../production/TaskEntry";
import type { ProductionController } from "../production/useProduction";
import type { Project } from "../model";

// Presentation only: generation records are never inserted into Agent history.
export function ConversationTimeline({
  canvas,
}: {
  canvas?: ProductionController;
  project: Project;
  settings: () => void;
}) {
  const messages = useAuiState((s) => s.thread.messages);
  const entries = [
    ...messages.map((message, index) => ({
      key: message.id,
      index,
      task: undefined,
      time: message.createdAt.getTime(),
    })),
    ...(canvas?.runs
      .filter((task) => task.turnId?.startsWith("direct:"))
      .map((task) => ({
        key: task.key,
        index: -1,
        task,
        time: task.createdAt ?? 0,
      })) ?? []),
  ].sort((a, b) => a.time - b.time);
  return entries.map((entry) =>
    entry.task && canvas ? (
      <TaskEntry key={entry.key} task={entry.task} canvas={canvas} />
    ) : (
      <ThreadPrimitive.MessageByIndex
        key={entry.key}
        index={entry.index}
        components={{
          UserMessage: AgentMessage,
          AssistantMessage: AgentMessage,
        }}
      />
    ),
  );
}
