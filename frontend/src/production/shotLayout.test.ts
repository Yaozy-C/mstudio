import { expect, test } from "bun:test";
import { fixture } from "./fixtures.test-helper";
import { productionItems } from "./items";
import {
  SHOT_CARD_WIDTH,
  SHOT_COLUMN_WIDTH,
  separateShotCards,
} from "./shotLayout";

test("triple-width shots leave room for frames and takes across consecutive shot columns", () => {
  const p = fixture();
  const shot = p.nodes.find((n) => n.shot)!;
  p.nodes.push({
    ...shot,
    id: "second",
    shot: { ...shot.shot!, order: 2, frames: [], takes: [] },
  });
  const items = productionItems(p);
  const first = items.find((i) => i.nodeId === shot.id && i.kind === "script")!;
  const second = items.find((i) => i.nodeId === "second")!;
  expect(first.width).toBe(460 * 3);
  expect(second.x - first.x).toBe(SHOT_COLUMN_WIDTH);
  const media = items.filter((i) => i.assetId);
  expect(media.every((i) => i.x >= first.x + SHOT_CARD_WIDTH + 24)).toBe(true);
  expect(media.every((i) => i.x + i.width < second.x)).toBe(true);
});

test("old cards covered by a wide shot are separated, while free manual positions stay fixed", () => {
  const p = fixture();
  p.production = {
    positions: {
      "node:frame-card-a": { x: 535, y: 110 },
      "node:frame-card-b": { x: 8000, y: 900 },
    },
  };
  const items = productionItems(p);
  const moved = items.find((i) => i.assetId === "a")!;
  expect(moved.x).toBeGreaterThan(SHOT_CARD_WIDTH);
  expect(items.find((i) => i.assetId === "b")).toMatchObject({
    x: 8000,
    y: 900,
  });
  expect(separateShotCards(items)).toEqual(items);
});
