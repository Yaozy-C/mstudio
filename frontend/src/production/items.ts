import { materializeFrameCards } from "./frameCards";
import { actionText } from "../creative/shotText";
import { takesOf } from "../creative/document";
import { framesOf } from "./frames";
import { itemKey, type ProductionItem } from "./types";
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
export function productionItems(p: Project): ProductionItem[] {
  p = materializeFrameCards(p);
  const items: ProductionItem[] = [];
  const assets = new Map(p.assets.map((asset) => [asset.id, asset]));
  const nodes = new Map(p.nodes.map((node) => [node.id, node]));
  const shots = productionShots(p);
  shots.forEach((n, i) => {
    const x = i * 1600,
      ownerId = n.id;
    const script = nodes
      .get(n.shot!.screenplayId)
      ?.screenplay?.script?.find((s) => s.id === n.shot!.scriptId);
    items.push({
      key: itemKey("script", n.id),
      kind: "script",
      ownerId,
      nodeId: n.id,
      title: n.title,
      text: [
        actionText(n) || script?.action,
        n.shot?.dialogue || script?.dialogue,
      ]
        .filter(Boolean)
        .join("\n\n"),
      x: x + 35,
      y: 110,
      width: 460,
      height: 300,
    });
    (n.references ?? []).forEach((ref, j) => {
      const a = assets.get(ref.assetId);
      if (!a) return;
      items.push({
        key: itemKey("reference", n.id, a.id),
        kind:
          a.kind === "video"
            ? "video"
            : a.kind === "image"
              ? "reference"
              : "note",
        ownerId,
        assetId: a.id,
        title: a.name,
        text: ref.purpose,
        x: x + 1135,
        y: 110 + j * 395,
        width: 250,
        height: 350,
      });
    });
    takesOf(n)
      .filter((t) => assets.get(t.assetId)?.kind === "video")
      .forEach((t, j) => {
        const a = assets.get(t.assetId)!;
        items.push({
          key: itemKey("take", n.id, a.id),
          kind: "video",
          ownerId,
          nodeId: n.id,
          assetId: a.id,
          title: a.name,
          text: n.text,
          x: x + 1135,
          y: 110 + j * 395,
          width: 260,
          height: 350,
        });
      });
  });
  p.nodes
    .filter((n) => !n.shot && n.kind !== "screenplay")
    .forEach((n, i) => {
      const a = assets.get(n.assetId ?? "");
      items.push({
        key: itemKey("node", n.id),
        usages: shots.flatMap((shot) =>
          framesOf(shot)
            .filter((f) => f.assetId === a?.id)
            .map((f) => ({ shotId: shot.id, title: f.title })),
        ),
        nodeId: n.id,
        assetId: a?.id,
        kind:
          a?.kind === "image"
            ? shots.some((shot) =>
                framesOf(shot).some((f) => f.assetId === a.id),
              )
              ? "image"
              : "reference"
            : a?.kind === "video"
              ? "video"
              : "note",
        title: n.title,
        text: n.text,
        x: shots.length ? (i % 4) * 300 : n.x,
        y: shots.length ? 1050 + Math.floor(i / 4) * 360 : n.y,
        width: a?.kind === "image" ? 250 : (n.width ?? 260),
        height: a?.kind === "image" ? 350 : (n.height ?? 310),
      });
    });
  const hidden = new Set(p.production?.hidden ?? []);
  return items
    .filter((n) => !hidden.has(n.key))
    .map((n) => ({ ...n, ...p.production?.positions?.[n.key] }));
}
