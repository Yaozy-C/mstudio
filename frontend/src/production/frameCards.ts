import type { Project } from "../model";
import { framesOf } from "./frames";
import { itemKey } from "./types";

// Legacy shot frames describe usage. Materialize a standalone card only when
// the image has no card yet; never collapse user-created copies.
export function materializeFrameCards(p: Project): Project {
  let next = p;
  const represented = new Set(
    p.nodes.filter((n) => !n.shot).map((n) => n.assetId),
  );
  p.nodes
    .filter((n) => n.shot)
    .forEach((shot, i) => {
      framesOf(shot).forEach((frame, j) => {
        if (
          represented.has(frame.assetId) ||
          !p.assets.some((a) => a.id === frame.assetId)
        )
          return;
        represented.add(frame.assetId);
        const legacyKey = itemKey("frame", shot.id, frame.assetId);
        let id = `frame-card-${frame.assetId}`;
        while (next.nodes.some((n) => n.id === id)) id += "-copy";
        const key = itemKey("node", id);
        const position = p.production?.positions?.[key] ??
          p.production?.positions?.[legacyKey] ?? {
            x: i * 1600 + 535 + (j % 2) * 290,
            y: 110 + Math.floor(j / 2) * 395,
          };
        next = {
          ...next,
          nodes: [
            ...next.nodes,
            {
              id,
              kind: "asset",
              assetId: frame.assetId,
              title: frame.title,
              text: frame.prompt ?? "",
              ...position,
            },
          ],
          production: {
            ...next.production,
            positions: { ...next.production?.positions, [key]: position },
            hidden: p.production?.hidden?.includes(legacyKey)
              ? [...(next.production?.hidden ?? []), key]
              : next.production?.hidden,
          },
        };
      });
    });
  return next;
}
