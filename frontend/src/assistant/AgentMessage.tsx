import { t, useLanguage } from "../i18n";
import { ErrorNotice } from "../errors/ErrorNotice";
import { TaskTarget } from "./TaskTarget";
import { createContext, useContext } from "react";
import {
  ActionBarPrimitive,
  MessagePrimitive,
  useAuiState,
} from "@assistant-ui/react";
import { MarkdownTextPrimitive } from "@assistant-ui/react-markdown";
import remarkGfm from "remark-gfm";
import type { Project } from "../model";
import type { Attachment } from "./attachments";
import { AgentActivity } from "./AgentActivity";
import { AttachmentChips } from "./AttachmentChips";
import { GenerationMessages } from "../production/GenerationMessages";
import type { ProductionTask } from "../production/types";
import type { ProductionController } from "../production/useProduction";
import type { AgentProfile } from "../agents/catalog";
// Stable component identity preserves Markdown nodes while streaming text grows.
function MessageText() {
  useLanguage();
  return <MarkdownTextPrimitive remarkPlugins={[remarkGfm]} />;
}
const contentComponents = { Text: MessageText };
export const MessageContext = createContext<{
  project: Project;
  agents: Pick<AgentProfile, "id" | "name">[];
  sent: Record<string, Attachment[]>;
  targets: Record<
    string,
    { agentId: string; agentName: string; targetNodeId?: string | null }
  >;
  canvas?: ProductionController;
  settings: () => void;
  follow: (
    task: ProductionTask,
    text?: string,
    kind?: "image" | "video",
  ) => void;
} | null>(null);
export function AgentMessage() {
  useLanguage();
  const message = useAuiState((s) => s.message);
  const context = useContext(MessageContext)!;
  const meta = message.metadata.custom;
  const agentName =
    (meta?.agentName as string | undefined) ||
    context.targets[message.id]?.agentName;
  const turnId = meta?.turnId as string | undefined;
  const attached =
    context.sent[message.id] ??
    (message.metadata.custom?.attachments as Attachment[] | undefined) ??
    [];
  return (
    <MessagePrimitive.Root className={`agent-message ${message.role}`}>
      <small>
        {message.role === "assistant" ? agentName || t("助手") : t("你")}
        {message.role === "user" && agentName && (
          <span className="message-agent-tag">@{agentName}</span>
        )}
      </small>
      {message.role === "assistant" && turnId && (
        <AgentActivity
          projectId={context.project.id}
          agents={context.agents}
          turnId={turnId}
          running={message.status?.type === "running"}
        />
      )}
      {message.role === "user" && (
        <TaskTarget
          project={context.project}
          id={
            context.targets[message.id]?.targetNodeId ??
            (meta?.taskTarget as { id?: string } | undefined)?.id
          }
        />
      )}
      <AttachmentChips items={attached} project={context.project} />
      <MessagePrimitive.Content components={contentComponents} />
      {message.role === "assistant" && turnId && (
        <GenerationMessages turnId={turnId} />
      )}
      {message.role === "assistant" &&
        message.status?.type === "incomplete" && (
          <ErrorNotice
            error={
              message.status.reason === "cancelled"
                ? t("已停止回答")
                : message.status.error || t("回答未完成")
            }
            fallback="CHAT_FAILED"
          >
            <ActionBarPrimitive.Reload className="text-button">
              {t("重试回答")}
            </ActionBarPrimitive.Reload>
          </ErrorNotice>
        )}
    </MessagePrimitive.Root>
  );
}
