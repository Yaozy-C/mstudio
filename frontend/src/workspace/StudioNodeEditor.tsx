import { CanvasPreview } from "../production/CanvasPreview";
import { productionItems } from "../production/items";
import { NodeEditor } from "../canvas/NodeEditor";
import { canvasViewport } from "../canvas/viewportMemory";
import type { Asset, BoardNode, Project } from "../model";
import { deleteCanvasNodes } from "./deleteAssets";
export function StudioNodeEditor({
  node,
  project,
  change,
  close,
  reference,
  add,
}: {
  node: BoardNode;
  project: Project;
  change: (fn: (p: Project) => Project) => void;
  close: () => void;
  reference: (id: string) => void;
  add: (asset: Asset) => void;
}) {
  const script = productionItems(project).find(
    (item) => item.nodeId === node.id && item.kind === "script",
  );
  if (script)
    return (
      <CanvasPreview
        onAdd={add}
        item={script}
        project={project}
        change={change}
        close={close}
      />
    );
  const asset = project.assets.find(
    (a) =>
      a.id ===
      (node.resultAssetId || node.assetId || node.references?.[0]?.assetId),
  );
  return (
    <NodeEditor
      node={node}
      asset={asset}
      view={canvasViewport(project)}
      onClose={close}
      onReference={() => reference(node.id)}
      onRemove={() => {
        change((p) => deleteCanvasNodes(p, [node.id]));
        close();
      }}
      onAdd={() => {
        if (asset) add(asset);
      }}
      onCommit={(title, text) =>
        change((p) => ({
          ...p,
          nodes: p.nodes.map((n) =>
            n.id === node.id ? { ...n, title, text } : n,
          ),
        }))
      }
    />
  );
}
