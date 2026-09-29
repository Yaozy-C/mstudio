import { expect, test } from "bun:test";
import { visibleItems } from "./visibleItems";

for (const count of [100, 300, 500]) {
  test(`${count} cards mount only the viewport and overscan`, () => {
    const cards = Array.from({ length: count }, (_, i) => ({
      id: i,
      x: (i % 20) * 300,
      y: Math.floor(i / 20) * 400,
      width: 260,
      height: 350,
    }));
    const size = { width: 1200, height: 800 };
    const visible = visibleItems(cards, { x: 0, y: 0, scale: 1 }, size);
    expect(visible.length).toBeLessThanOrEqual(15);
    expect(visible.map((v) => v.id)).toContain(0);
    const panned = visibleItems(cards, { x: -3000, y: 0, scale: 1 }, size);
    expect(panned.map((v) => v.id)).toContain(10);
    expect(panned.map((v) => v.id)).not.toContain(0);
    expect(
      visibleItems(cards, { x: 0, y: 0, scale: 0.18 }, size).length,
    ).toBeGreaterThan(visible.length);
  });
}
test("viewport intersection includes partially visible large text cards and respects zoom", () => {
  const cards = [{ x: -200, y: -200, width: 500, height: 2000 }];
  expect(
    visibleItems(
      cards,
      { x: 0, y: -1000, scale: 1 },
      { width: 800, height: 600 },
    ),
  ).toHaveLength(1);
  expect(
    visibleItems(cards, { x: 0, y: 0, scale: 1 }, { width: 0, height: 0 }),
  ).toEqual([]);
});
