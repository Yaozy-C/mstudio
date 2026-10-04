import type { ThreadMessageLike } from "@assistant-ui/react";

type State = { isRunning: boolean; messages: readonly ThreadMessageLike[] };

// Ignore runtime-generated IDs: persisted and live messages use different IDs.
function signature(messages: readonly ThreadMessageLike[]) {
  return JSON.stringify(
    messages.map((m) => ({
      role: m.role,
      turn: m.metadata?.custom?.turnId,
      text:
        typeof m.content === "string"
          ? m.content
          : m.content
              .filter((part) => part.type === "text")
              .map((part) => part.text)
              .join(""),
      status: m.status?.type === "incomplete" ? m.status : undefined,
    })),
  );
}

export function historySync(options: {
  state: () => State;
  read: () => Promise<ThreadMessageLike[]>;
  apply: (messages: ThreadMessageLike[]) => void;
  error: (error: unknown) => void;
}) {
  let revision = 0;
  let disposed = false;
  return {
    invalidate() {
      revision++;
    },
    dispose() {
      disposed = true;
      revision++;
    },
    async refresh() {
      const request = ++revision;
      const before = options.state();
      if (disposed || before.isRunning) return;
      try {
        const messages = await options.read();
        const current = options.state();
        if (
          disposed ||
          request !== revision ||
          current.isRunning ||
          current.messages !== before.messages
        )
          return;
        // A failed local send may not have reached persistence. Keep it retryable.
        const turns = new Set(messages.map((m) => m.metadata?.custom?.turnId));
        const recent = current.messages.slice(-messages.length);
        if (
          recent.some(
            (m) =>
              m.metadata?.custom?.turnId &&
              !turns.has(m.metadata.custom.turnId),
          )
        )
          return;
        if (signature(recent) !== signature(messages)) options.apply(messages);
      } catch (error) {
        if (!disposed && request === revision) options.error(error);
      }
    },
  };
}
