import { useMemo, useState, type ReactNode } from "react";
import { ThreadPrimitive, useAuiEvent, useAuiState } from "@assistant-ui/react";
import { AgentMessage } from "./AgentMessage";
import { TaskEntry } from "../production/TaskEntry";
import type { ProductionController } from "../production/useProduction";
import { VirtualConversation } from "./VirtualConversation";

// Presentation only: generation records are never inserted into Agent history.
export function ConversationTimeline({
  canvas,
  children,
}: {
  canvas?: ProductionController;
  children?: ReactNode;
}) {
  const [viewport, setViewport] = useState<HTMLDivElement | null>(null);
  const messages = useAuiState((s) => s.thread.messages);
  const [scrollRequest, setScrollRequest] = useState(0);
  useAuiEvent("thread.runStart", () =>
    setScrollRequest((request) => request + 1),
  );
  const entries = useMemo(
    () =>
      [
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
      ].sort((a, b) => a.time - b.time),
    [messages, canvas?.runs],
  );
  return (
    <ThreadPrimitive.Viewport
      ref={setViewport}
      className="agent-messages"
      autoScroll={false}
      scrollToBottomOnInitialize={false}
      scrollToBottomOnRunStart={false}
      scrollToBottomOnThreadSwitch={false}
    >
      {children}
      <VirtualConversation
        entries={entries}
        viewport={viewport}
        scrollRequest={scrollRequest}
      >
        {(entry) =>
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
          )
        }
      </VirtualConversation>
    </ThreadPrimitive.Viewport>
  );
}
