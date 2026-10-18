import { useLayoutEffect, useRef } from "react";
import type { PrevisEditorState } from "./usePrevisEditor";

/** Follow new output only while the reader remains at the bottom. */
export function usePrevisScroll(state: PrevisEditorState) {
  const viewport = useRef<HTMLDivElement>(null);
  const following = useRef(true);
  const previousPrompt = useRef(state.pendingPrompt);
  useLayoutEffect(() => {
    if (state.pendingPrompt && state.pendingPrompt !== previousPrompt.current)
      following.current = true;
    previousPrompt.current = state.pendingPrompt;
    const element = viewport.current;
    if (element && following.current) element.scrollTop = element.scrollHeight;
  }, [
    state.messages.length,
    state.busy,
    state.pendingPrompt,
    state.agentPartial,
    state.agentEvents,
  ]);
  return {
    ref: viewport,
    onScroll: () => {
      const element = viewport.current;
      if (element)
        following.current =
          element.scrollHeight - element.clientHeight - element.scrollTop <= 2;
    },
    onWheelCapture: (event: { deltaY: number }) => {
      if (event.deltaY < 0) following.current = false;
    },
  };
}
