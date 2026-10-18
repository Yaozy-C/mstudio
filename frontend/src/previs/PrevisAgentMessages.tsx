import type { ReactNode } from "react";
import Markdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { AgentMark } from "../ui/Identity";
import { AgentActivityFeed } from "../assistant/AgentActivityFeed";
import { MessageCopyButton } from "../assistant/MessageCopyButton";
import { t } from "../i18n";
import type { PrevisEditorState } from "./usePrevisEditor";
import "../styles/agent-activity.css";

function Message({
  role,
  text,
  children,
}: {
  role: string;
  text: string;
  children?: ReactNode;
}) {
  return (
    <div className={`agent-message ${role}`}>
      <div className="agent-message-header">
        {role !== "user" && (
          <AgentMark id="director" size={20} className="agent-message-icon" />
        )}
        <span>{role === "user" ? t("你") : t("预演 Agent")}</span>
      </div>
      <div className="agent-message-body">
        {children}
        {text && (
          <div className="agent-message-text">
            <Markdown remarkPlugins={[remarkGfm]}>{text}</Markdown>
          </div>
        )}
        {text && role !== "user" && <MessageCopyButton text={text} />}
      </div>
    </div>
  );
}

export function PrevisAgentMessages({ state }: { state: PrevisEditorState }) {
  const {
    messages,
    pendingPrompt,
    agentPartial = "",
    agentEvents = [],
    seconds,
    busy,
    phase,
    error,
  } = state;
  const running = busy === "chat" || busy === "render";
  const completed = phase === "complete" && !running && !error;
  const activity = agentEvents.length > 0 && (
    <AgentActivityFeed
      events={agentEvents}
      seconds={seconds}
      running={running && phase === "generating"}
    />
  );
  return (
    <>
      {messages.map((message, index) => (
        <Message key={index} role={message.role} text={message.text}>
          {completed &&
            index === messages.length - 1 &&
            message.role === "assistant" &&
            activity}
        </Message>
      ))}
      {pendingPrompt && <Message role="user" text={pendingPrompt} />}
      {!completed && (activity || agentPartial) && (
        <Message role="assistant" text={agentPartial}>
          {activity}
        </Message>
      )}
    </>
  );
}
