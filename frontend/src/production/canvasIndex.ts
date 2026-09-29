import type { BoardNode, Clip } from "../model";
import type { ProductionItem } from "./types";

// Membership is a property of the current edit, never a saved approval flag.
export function timelineAssetIds(clips: Clip[]): Set<string> {
  return new Set(clips.map((clip) => clip.assetId));
}

export function shotAreas(shots: BoardNode[], items: ProductionItem[]) {
  const bottoms = new Map<string, number>();
  for (const item of items)
    if (item.ownerId)
      bottoms.set(
        item.ownerId,
        Math.max(bottoms.get(item.ownerId) ?? 680, item.y + item.height + 30),
      );
  return shots.map((shot, index) => ({
    ...shot,
    index,
    x: index * 1600,
    y: 0,
    width: 1600,
    height: bottoms.get(shot.id) ?? 680,
  }));
}
