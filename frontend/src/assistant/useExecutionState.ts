import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { bridge, native } from "../bridge";
import { executionSync, type ExecutionState } from "./executionState";

export function useExecutionState(
  projectId: string,
  onError: (message: string) => void,
) {
  const [state, setState] = useState<ExecutionState | null>(null);
  useEffect(() => {
    setState(null);
    if (!native) return;
    const sync = executionSync({
      read: () =>
        bridge<ExecutionState | null>("agent_execution_state", { projectId }),
      apply: setState,
      error: (error) => onError(String(error)),
    });
    const subscription = listen<ExecutionState>(
      "agent-execution",
      ({ payload }) => {
        if (payload.projectId === projectId) sync.receive(payload);
      },
    );
    void subscription
      .then(() => sync.refresh())
      .catch((error) => onError(String(error)));
    const refresh = () => {
      void sync.refresh();
    };
    window.addEventListener("focus", refresh);
    const reveal = () => {
      if (document.visibilityState === "visible") refresh();
    };
    document.addEventListener("visibilitychange", reveal);
    return () => {
      sync.dispose();
      window.removeEventListener("focus", refresh);
      document.removeEventListener("visibilitychange", reveal);
      void subscription.then((off) => off());
    };
  }, [projectId, onError]);
  return state;
}
