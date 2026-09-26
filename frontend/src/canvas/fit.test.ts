import { expect, test } from "bun:test";
import { fitView } from "./fit";
import { nodeSize } from "./geometry";
import type { BoardNode } from "../model";

test("fit uses the available docked canvas and keeps every node above its controls", () => {
  const nodes: BoardNode[] = [
    { id: "a", kind: "text", title: "A", text: "", x: -200, y: 400 },
    { id: "b", kind: "asset", title: "B", text: "", x: 600, y: 850 },
  ];
  for (const size of [
    { width: 700, height: 452 },
    { width: 1536, height: 900 },
  ]) {
    const view = fitView(nodes, size);
    for (const node of nodes) {
      const bounds = nodeSize(node);
      expect(node.x * view.scale + view.x).toBeGreaterThanOrEqual(31);
      expect(node.y * view.scale + view.y).toBeGreaterThanOrEqual(31);
      expect((node.x + bounds.width) * view.scale + view.x).toBeLessThanOrEqual(
        size.width - 31,
      );
      expect(
        (node.y + bounds.height) * view.scale + view.y,
      ).toBeLessThanOrEqual(size.height - 67);
    }
  }
  expect(fitView(nodes, { width: 1536, height: 900 }).scale).toBe(1);
});

test("empty docked canvas starts within its own content coordinates", () => {
  expect(fitView([], { width: 700, height: 452 })).toEqual({
    x: 32,
    y: 32,
    scale: 1,
  });
});

test("locating a portrait result uses its dimensions and fits above the task panel", () => {
  const node: BoardNode = {
    id: "result",
    kind: "asset",
    title: "Result",
    text: "",
    x: -1800,
    y: 950,
    width: 250,
    height: 460,
  };
  const available = { width: 960, height: 420 };
  const view = fitView([node], available);
  const left = node.x * view.scale + view.x;
  const top = node.y * view.scale + view.y;
  expect(left + (node.width! * view.scale) / 2).toBeCloseTo(
    available.width / 2,
  );
  expect(top).toBeGreaterThanOrEqual(31);
  expect(top + node.height! * view.scale).toBeLessThanOrEqual(
    available.height - 67,
  );
  expect(view.scale).toBeLessThan(0.85);
});
