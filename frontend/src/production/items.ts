import { mediaCardSize } from "./mediaCardSize";
import { itemKey, type DraftItem } from "./types";
import type { Project } from "../model";
export function productionShots(p: Project) {
  const indices = new Map(p.nodes.map((node, i) => [node.id, i]));
  return p.nodes
    .filter((n) => n.shot)
    .sort((a, b) => {
      const group =
        (indices.get(a.shot!.screenplayId) ?? -1) -
        (indices.get(b.shot!.screenplayId) ?? -1);
      return group || a.shot!.order - b.shot!.order;
    });
}
export function draftItems(p: Project): DraftItem[] {
  return p.nodes
    .filter((n) => !n.shot && n.kind !== "screenplay" && n.kind !== "shot")
    .map((n) => {
      const a = p.assets.find((a) => a.id === n.assetId);
      return {
        key: itemKey("node", n.id),
        nodeId: n.id,
        assetId: a?.id,
        kind:
          a?.kind === "image"
            ? "image"
            : a?.kind === "video"
              ? "video"
              : "note",
        title: n.title,
        text: n.text,
        x: n.x,
        y: n.y,
        ...(a && ["image", "video"].includes(a.kind)
          ? mediaCardSize(a)
          : { width: n.width ?? 260, height: n.height ?? 310 }),
        ...p.production?.positions?.[itemKey("node", n.id)],
      };
    });
}
