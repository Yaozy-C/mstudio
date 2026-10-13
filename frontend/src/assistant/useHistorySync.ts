import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import type { AssistantRuntime } from "@assistant-ui/react";
import { bridge, native } from "../bridge";
import { historyMessages, type HistoryMessage } from "./historyMessages";
import { historySync } from "./historySync";
import type { AgentProgress } from "./streamReply";

export function useHistorySync(
  projectId: string,
  runtime: AssistantRuntime,
  visible: boolean,
  view: string | undefined,
  onError: (message: string) => void,
) {
  useEffect(() => {
    if (!native) return;
    const sync = historySync({
      state: () => runtime.thread.getState(),
      read: async () =>
        historyMessages(
          await bridge<HistoryMessage[]>("agent_history", { projectId }),
        ),
      apply: (messages) => runtime.thread.reset(messages),
      error: (error) => onError(String(error)),
    });
    const refresh = () => {
      void sync.refresh();
    };
    let running = runtime.thread.getState().isRunning;
    const unsubscribe = runtime.thread.subscribe(() => {
      const next = runtime.thread.getState().isRunning;
      if (next === running) return;
      running = next;
      if (next) sync.invalidate();
      else refresh();
    });
    let disposed = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let off: (() => void) | undefined;
    void listen<AgentProgress>("agent-progress", ({ payload }) => {
      if (payload.projectId !== projectId) return;
      if (["turn/start", "turn/waiting", "turn/end"].includes(payload.kind)) {
        clearTimeout(timer);
        timer = undefined;
        refresh();
      } else if (
        payload.kind === "assistant/partial" &&
        !runtime.thread.getState().isRunning &&
        timer === undefined
      ) {
        timer = setTimeout(() => {
          timer = undefined;
          refresh();
        }, 100);
      }
    })
      .then((stop) => {
        if (disposed) stop();
        else {
          off = stop;
          refresh();
        }
      })
      .catch((error) => {
        if (!disposed) onError(String(error));
      });
    window.addEventListener("focus", refresh);
    const reveal = () => {
      if (document.visibilityState === "visible") refresh();
    };
    document.addEventListener("visibilitychange", reveal);
    return () => {
      disposed = true;
      clearTimeout(timer);
      sync.dispose();
      off?.();
      unsubscribe();
      window.removeEventListener("focus", refresh);
      document.removeEventListener("visibilitychange", reveal);
    };
  }, [projectId, runtime, visible, view, onError]);
}
