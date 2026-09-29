import { useCreativeEvents } from "../creative/useCreativeEvents";
import type { Dispatch, SetStateAction } from "react";
import type { initialPanels } from "./studioPanels";
export function useStudioCreativeEvents(
  clock: { pause: () => void },
  setPanels: Dispatch<SetStateAction<ReturnType<typeof initialPanels>>>,
  setEditing: (id: null) => void,
  setDialog: (kind: null) => void,
  setClipId: (id: null) => void,
  setNodeId: (id: null) => void,
) {
  useCreativeEvents({
    assist: () => {
      setPanels((p) => ({ ...p, agent: true, inspector: false }));
      setEditing(null);
      setDialog(null);
    },
    reading: () => {
      clock.pause();
      setClipId(null);
      setNodeId(null);
      setPanels((p) => ({ ...p, preview: false, timeline: false }));
    },
    arranged: () => setPanels((p) => ({ ...p, timeline: true, preview: true })),
  });
}
