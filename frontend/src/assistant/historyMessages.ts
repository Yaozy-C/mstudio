import type { ThreadMessageLike } from "@assistant-ui/react";
import { historyAttachments } from "./attachments";
export type HistoryMessage = {
  id: number;
  role: "user" | "assistant";
  content: string;
  payload: unknown;
  attribution?: Record<string, unknown>;
};
export function historyMessages(
  messages: HistoryMessage[],
): ThreadMessageLike[] {
  return messages.map((m) => {
    const status = m.attribution?.status;
    const incomplete = [
      "failed",
      "cancelled",
      "interrupted",
      "running",
    ].includes(String(status));
    return {
      id: `history-${m.id}`,
      role: m.role,
      createdAt: new Date(
        typeof m.attribution?.createdAt === "number"
          ? m.attribution.createdAt
          : 0,
      ),
      content: m.content,
      ...(m.role === "assistant" && incomplete
        ? {
            status: {
              type: "incomplete" as const,
              reason:
                status === "cancelled"
                  ? ("cancelled" as const)
                  : ("error" as const),
              error:
                typeof m.attribution?.error === "string"
                  ? m.attribution.error
                  : "上次回答未完成，原消息和附件已保留，可重试。",
            },
          }
        : {}),
      metadata: {
        custom: {
          attachments: historyAttachments(m.payload),
          taskTarget: Array.isArray(m.payload)
            ? m.payload[0]?.taskTarget
            : undefined,
          ...m.attribution,
        },
      },
    };
  });
}
