import { materializeFrameCards } from "./frameCards";
import { mediaCardSize } from "./mediaCardSize";
import { actionText } from "../creative/shotText";
import { takesOf } from "../creative/document";
import { framesOf } from "./frames";
import { itemKey, type ProductionItem } from "./types";
import type { Project } from "../model";
import {
  SHOT_CARD_WIDTH,
  SHOT_COLUMN_WIDTH,
  SHOT_VIDEO_OFFSET,
  separateShotCards,
} from "./shotLayout";
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
    const x = i * SHOT_COLUMN_WIDTH,
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
      width: SHOT_CARD_WIDTH,
      height: 300,
    });
    let videoY = 110;
    takesOf(n)
      .filter((t) => assets.get(t.assetId)?.kind === "video")
      .forEach((t) => {
        const a = assets.get(t.assetId)!;
        const size = mediaCardSize(a);
        items.push({
          key: itemKey("take", n.id, a.id),
          kind: "video",
          ownerId,
          nodeId: n.id,
          assetId: a.id,
          title: a.name,
          text: n.text,
          x: x + SHOT_VIDEO_OFFSET,
          y: videoY,
          ...size,
        });
        videoY += size.height + 45;
      });
  });
  let rowY = 1050;
  let rowHeight = 0;
  p.nodes
    .filter((n) => !n.shot && n.kind !== "screenplay")
    .forEach((n, i) => {
      const a = assets.get(n.assetId ?? "");
      const size =
        a && ["image", "video"].includes(a.kind)
          ? mediaCardSize(a)
          : { width: n.width ?? 260, height: n.height ?? 310 };
      if (i > 0 && i % 4 === 0) {
        rowY += rowHeight + 45;
        rowHeight = 0;
      }
      rowHeight = Math.max(rowHeight, size.height);
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
        y: shots.length ? rowY : n.y,
        ...size,
      });
    });
  const hidden = new Set(p.production?.hidden ?? []);
  return separateShotCards(
    items
      .filter((n) => !hidden.has(n.key))
      .map((n) => ({ ...n, ...p.production?.positions?.[n.key] })),
  );
}
