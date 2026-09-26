import { test, expect } from "bun:test";
import { fixture } from "./fixtures.test-helper";
import { productionItems } from "./items";
import { materializeFrameCards } from "./frameCards";

test("frame usage reuses an independent card and preserves deliberate copies", () => {
  const p = fixture();
  p.nodes.push({
    id: "standalone",
    kind: "asset",
    assetId: "a",
    title: "My image",
    text: "",
    x: 1,
    y: 2,
  });
  const next = materializeFrameCards(p);
  expect(next.nodes.filter((n) => n.assetId === "a")).toHaveLength(1);
  expect(productionItems(next).filter((n) => n.assetId === "a")).toHaveLength(
    1,
  );
  expect(
    productionItems(next).find((n) => n.nodeId === "standalone")?.usages
      ?.length,
  ).toBe(1);
  next.nodes.push({
    ...next.nodes.find((n) => n.id === "standalone")!,
    id: "my-copy",
  });
  expect(productionItems(next).filter((n) => n.assetId === "a")).toHaveLength(
    2,
  );
  expect(materializeFrameCards(next)).toBe(next);
});

test("legacy frames become stable cards and removing usage does not remove the image", () => {
  const next = materializeFrameCards(fixture());
  const card = next.nodes.find((n) => n.assetId === "a")!;
  const before = productionItems(next).find((n) => n.nodeId === card.id)!;
  const unlinked = {
    ...next,
    nodes: next.nodes.map((n) =>
      n.shot ? { ...n, shot: { ...n.shot, frames: [] } } : n,
    ),
  };
  const after = productionItems(unlinked).find((n) => n.nodeId === card.id)!;
  expect(after.key).toBe(before.key);
  expect([after.x, after.y]).toEqual([before.x, before.y]);
  expect(after.usages).toEqual([]);
});
