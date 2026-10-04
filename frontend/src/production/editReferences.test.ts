import { expect, test } from "bun:test";
import { fixture, asset } from "./fixtures.test-helper";
import { applyOperations } from "../assistant/projectCommands";
import { productionItems } from "./items";
import { editReferences } from "./editReferences";

const edit = (p: ReturnType<typeof fixture>, extra: object) =>
  applyOperations(p, [{ op: "set_references", id: "shot", ...extra }]);
test("adding and editing one reference preserves other references and never creates cards", () => {
  const p = fixture();
  const next = edit(p, {
    referenceMode: "upsert",
    references: [{ assetId: "a", purpose: "wardrobe" }],
  });
  const updated = edit(next, {
    referenceMode: "upsert",
    references: [{ assetId: "a", purpose: "identity" }],
  });
  expect(updated.nodes[1].references).toEqual([
    { assetId: "reference", purpose: "product" },
    { assetId: "a", purpose: "identity" },
  ]);
  expect(updated.assets).toBe(p.assets);
  expect(updated.nodes.length).toBe(p.nodes.length);
  expect(productionItems(updated).some((i) => i.assetId === "reference")).toBe(
    false,
  );
  const removed = edit(updated, { referenceMode: "remove", assetIds: ["a"] });
  expect(removed.nodes[1].references).toEqual(p.nodes[1].references);
  expect(removed.assets).toBe(p.assets);
  expect(removed.nodes[1].shot!.frames).toEqual(p.nodes[1].shot!.frames);
});
test("reference edits reject missing media and total overflow atomically", () => {
  const p = fixture();
  expect(() =>
    edit(p, {
      referenceMode: "upsert",
      references: [{ assetId: "missing", purpose: "test" }],
    }),
  ).toThrow();
  p.assets.push(...Array.from({ length: 12 }, (_, i) => asset(`new-${i}`)));
  expect(() =>
    edit(p, {
      referenceMode: "upsert",
      references: p.assets
        .slice(-12)
        .map((a) => ({ assetId: a.id, purpose: "test" })),
    }),
  ).toThrow();
  expect(p.nodes[1].references).toHaveLength(1);
});
test("local editor saves preserve concurrent additions and explicit replacement still works", () => {
  const p = fixture();
  const before = p.nodes[1].references!;
  const concurrent = edit(p, {
    referenceMode: "upsert",
    references: [{ assetId: "a", purpose: "new" }],
  });
  const saved = editReferences(concurrent, "shot", before, []);
  expect(saved.nodes[1].references).toEqual([{ assetId: "a", purpose: "new" }]);
  expect(
    edit(saved, { referenceMode: "replace", references: [] }).nodes[1]
      .references,
  ).toEqual([]);
});
test("manual media cards remain while multiple shots reference the same asset", () => {
  const p = fixture();
  p.nodes.push({
    ...p.nodes[1],
    id: "second",
    shot: { ...p.nodes[1].shot!, order: 2 },
  });
  p.nodes.push({
    id: "manual",
    kind: "asset",
    assetId: "reference",
    title: "Product",
    text: "",
    x: 0,
    y: 0,
  });
  expect(
    productionItems(p)
      .filter((i) => i.assetId === "reference")
      .map((i) => i.nodeId),
  ).toEqual(["manual"]);
});

test("updating video reference purpose preserves its source interval", () => {
  const p = fixture();
  p.nodes[1].references = [
    { assetId: "v", purpose: "motion", start: 2, end: 4 },
  ];
  const next = edit(p, {
    referenceMode: "upsert",
    references: [{ assetId: "v", purpose: "camera" }],
  });
  expect(next.nodes[1].references).toEqual([
    { assetId: "v", purpose: "camera", start: 2, end: 4 },
  ]);
});
