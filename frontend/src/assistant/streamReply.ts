import type { bridge } from "../bridge";
export type AgentProgress = {
  projectId: string;
  turnId: string;
  kind: string;
  payload?: { text?: string };
};
export type SubscribeProgress = (
  receive: (event: AgentProgress) => void,
) => Promise<() => void>;

// Subscribe before invoking: even a fast first frame must survive the IPC race.
export async function* streamReply(
  invoke: typeof bridge,
  request: Record<string, unknown>,
  signal: AbortSignal,
  subscribe?: SubscribeProgress,
): AsyncGenerator<string> {
  let latest: string | undefined;
  let dirty = false;
  let finished = false;
  let failure: unknown;
  let failed = false;
  let wake: (() => void) | undefined;
  const off = await subscribe?.((event) => {
    if (
      event.projectId !== request.projectId ||
      event.turnId !== request.clientTurnId ||
      event.kind !== "assistant/partial" ||
      typeof event.payload?.text !== "string"
    )
      return;
    latest = event.payload.text;
    dirty = true;
    wake?.();
  });
  try {
    if (signal.aborted) throw new Error("已停止");
    // Catch immediately: a rejection must wake the consumer without an unhandled promise.
    const pending = invoke<string>("assistant_chat", { request })
      .then(
        (text) => {
          latest = text;
          dirty = true;
        },
        (error: unknown) => {
          failed = true;
          failure = error;
        },
      )
      .finally(() => {
        finished = true;
        wake?.();
      });
    while (!finished || dirty) {
      if (dirty) {
        dirty = false;
        if (latest !== undefined) yield latest;
      } else {
        await new Promise<void>((resolve) => {
          wake = resolve;
        });
        wake = undefined;
      }
    }
    await pending;
    if (failed) throw failure;
  } finally {
    off?.();
  }
}
