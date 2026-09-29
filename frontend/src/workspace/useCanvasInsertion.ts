import type { Project, Asset } from "../model";
import { canvasAsset, canvasNote } from "./canvasAsset";
import type { StudioView } from "./StudioStage";
import type { Dispatch, SetStateAction } from "react";
import type { initialPanels } from "./studioPanels";
export function useCanvasInsertion(
  project: Project,
  change: (fn: (p: Project) => Project) => void,
  setView: (view: StudioView) => void,
  setPanels: Dispatch<SetStateAction<ReturnType<typeof initialPanels>>>,
  selectNode: (id: string) => void,
  setEditing: (id: string) => void,
) {
  const place = (asset: Asset) => {
    setView("storyboard");
    setPanels((p) => ({ ...p, preview: false }));
    change((p) => ({
      ...p,
      nodes: [...p.nodes, canvasAsset(asset, p.nodes.length)],
    }));
  };
  function addNote(kind: "text" | "shot", text = "") {
    setView("storyboard");
    setPanels((p) => ({ ...p, preview: false }));
    const node = canvasNote(project, kind, text);
    change((p) => ({ ...p, nodes: [...p.nodes, node] }));
    selectNode(node.id);
    if (!text) setEditing(node.id);
  }
  return { place, addNote };
}
