import type { ProductionItem } from "./types";

export const SHOT_CARD_WIDTH = 460 * 3;
export const SHOT_COLUMN_WIDTH = SHOT_CARD_WIDTH + 1140;
export const SHOT_FRAME_OFFSET = SHOT_CARD_WIDTH + 75;
export const SHOT_VIDEO_OFFSET = SHOT_CARD_WIDTH + 675;

const overlaps = (a: ProductionItem, b: ProductionItem) =>
  a.x < b.x + b.width + 24 &&
  a.x + a.width + 24 > b.x &&
  a.y < b.y + b.height + 24 &&
  a.y + a.height + 24 > b.y;

/** Keep old media positions unless the wider shot would cover them. */
export function separateShotCards(items: ProductionItem[]) {
  const shots = items.filter((i) => i.kind === "script");
  const moving = new Set(
    items
      .filter((i) => i.kind !== "script" && shots.some((s) => overlaps(i, s)))
      .map((i) => i.key),
  );
  const placed = items.filter((i) => !moving.has(i.key));
  return items.map((item) => {
    if (!moving.has(item.key)) return item;
    const next = { ...item };
    let shot: ProductionItem | undefined;
    while ((shot = shots.find((s) => overlaps(next, s))))
      next.x = shot.x + shot.width + 32;
    while (placed.some((i) => overlaps(next, i))) next.y += next.height + 32;
    placed.push(next);
    return next;
  });
}
