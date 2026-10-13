import type { bridge } from "../bridge";
export type AgentProgress = {
  projectId: string;
  turnId: string;
  kind: string;
  payload?: { text?: string; status?: string; error?: string | null };
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
  let returned = false;
  let parked = false;
  let continued = false;
  let terminal: AgentProgress["payload"] | undefined;
  let wake: (() => void) | undefined;
  const off = await subscribe?.((event) => {
    if (
      event.projectId !== request.projectId ||
      event.turnId !== request.clientTurnId
    )
      return;
    if (event.kind === "turn/waiting") {
      parked = true;
    } else if (event.kind === "turn/end") {
      terminal = event.payload ?? { status: "completed" };
      if (returned) finished = true;
    } else if (
      event.kind === "assistant/partial" &&
      typeof event.payload?.text === "string"
    ) {
      latest = event.payload.text;
      dirty = true;
      if (parked) continued = true;
    }
    wake?.();
  });
  const abort = () => {
    failed = true;
    failure = new DOMException("已停止", "AbortError");
    finished = true;
    wake?.();
  };
  try {
    if (signal.aborted) throw new Error("已停止");
    signal.addEventListener("abort", abort, { once: true });
    // Catch immediately: a rejection must wake the consumer without an unhandled promise.
    const pending = invoke<string | { text: string; status: string }>(
      "assistant_chat",
      { request },
    )
      .then(
        (reply) => {
          // A fast background continuation may finish before the initial IPC returns.
          if (!terminal && !continued) {
            latest = typeof reply === "string" ? reply : reply.text;
            dirty = true;
          }
          returned = true;
          finished =
            typeof reply === "string" ||
            reply.status !== "waiting" ||
            !!terminal;
        },
        (error: unknown) => {
          failed = true;
          failure = error;
          returned = true;
          finished = true;
        },
      )
      .finally(() => {
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
    if (terminal?.status === "cancelled")
      throw new DOMException("已停止", "AbortError");
    if (terminal?.status === "failed")
      throw new Error(terminal.error || "回答未完成");
  } finally {
    signal.removeEventListener("abort", abort);
    off?.();
  }
}
