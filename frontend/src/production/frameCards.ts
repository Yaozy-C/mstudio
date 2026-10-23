import type { Project } from "../model";
import { framesOf } from "./frames";
import { mediaCardSize } from "./mediaCardSize";
import { itemKey } from "./types";
import { SHOT_COLUMN_WIDTH, SHOT_FRAME_OFFSET } from "./shotLayout";

// Shot frames describe usage. Materialize a standalone card only when
// the image has no card yet; never collapse user-created copies.
export function materializeFrameCards(p: Project): Project {
  let next = p;
  const represented = new Set(
    p.nodes.filter((n) => !n.shot).map((n) => n.assetId),
  );
  p.nodes
    .filter((n) => n.shot)
    .forEach((shot, i) => {
      let rowY = 110;
      let rowHeight = 0;
      framesOf(shot).forEach((frame, j) => {
        if (j > 0 && j % 2 === 0) {
          rowY += rowHeight + 45;
          rowHeight = 0;
        }
        const asset = p.assets.find((a) => a.id === frame.assetId);
        if (asset) rowHeight = Math.max(rowHeight, mediaCardSize(asset).height);
        if (
          represented.has(frame.assetId) ||
          !p.assets.some((a) => a.id === frame.assetId)
        )
          return;
        represented.add(frame.assetId);
        let id = `frame-card-${frame.assetId}`;
        while (next.nodes.some((n) => n.id === id)) id += "-copy";
        const key = itemKey("node", id);
        const position = p.production?.positions?.[key] ?? {
          x: i * SHOT_COLUMN_WIDTH + SHOT_FRAME_OFFSET + (j % 2) * 290,
          y: rowY,
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
              text: "",
              ...position,
            },
          ],
          production: {
            ...next.production,
            positions: { ...next.production?.positions, [key]: position },
          },
        };
      });
    });
  return next;
}
