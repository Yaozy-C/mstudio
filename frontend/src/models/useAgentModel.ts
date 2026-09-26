import { useRef, useState } from "react";
import { useModels } from "./useModels";
import { readyModel, selectedModel } from "./types";
export function useAgentModel(projectId: string) {
  const hub = useModels(projectId);
  const [running, setRunning] = useState(false);
  const selected = selectedModel(hub.catalog);
  const current = useRef(selected);
  current.current = selected;
  return {
    ...hub,
    current,
    selected,
    running,
    setRunning,
    error: hub.error,
    available: !!selected && readyModel(selected) && !hub.loading && !hub.error,
  };
}
