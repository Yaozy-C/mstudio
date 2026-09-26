import { useEffect, useState } from "react";
import type { ThreadMessageLike } from "@assistant-ui/react";
import { bridge, native } from "../bridge";
import { historyMessages, type HistoryMessage } from "./historyMessages";
export function useAgentHistory(projectId: string) {
  const [history, setHistory] = useState<ThreadMessageLike[] | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    if (!native) {
      setHistory([]);
      return;
    }
    let active = true;
    void bridge<HistoryMessage[]>("agent_history", { projectId })
      .then((v) => {
        if (active) setHistory(historyMessages(v));
      })
      .catch((e) => setError(String(e)));
    return () => {
      active = false;
    };
  }, [projectId]);
  return { history, error };
}
