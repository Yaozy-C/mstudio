import { useEffect } from "react";
import type { AssistantRuntime } from "@assistant-ui/react";

export function useComposerDraft(projectId: string, runtime: AssistantRuntime) {
  useEffect(() => {
    const key = `mstudio-chat-draft:${projectId}`;
    const saved = localStorage.getItem(key);
    if (saved && !runtime.thread.composer.getState().text)
      runtime.thread.composer.setText(saved);
    return runtime.thread.composer.subscribe(() => {
      const text = runtime.thread.composer.getState().text;
      if (text) localStorage.setItem(key, text);
      else localStorage.removeItem(key);
    });
  }, [projectId, runtime]);
}
