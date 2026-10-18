import { expect, test } from "bun:test";
import { fixture, asset } from "./fixtures.test-helper";
import { applyOperations } from "../assistant/projectCommands";
import { productionItems } from "./items";
import { editReferences, mergeReferenceDraft } from "./editReferences";

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
test("reference edits reject missing media and allow more than twelve references", () => {
  const p = fixture();
  expect(() =>
    edit(p, {
      referenceMode: "upsert",
      references: [{ assetId: "missing", purpose: "test" }],
    }),
  ).toThrow();
  p.assets.push(...Array.from({ length: 12 }, (_, i) => asset(`new-${i}`)));
  const next = edit(p, {
    referenceMode: "upsert",
    references: p.assets
      .slice(-12)
      .map((a) => ({ assetId: a.id, purpose: "test" })),
  });
  expect(next.nodes[1].references).toHaveLength(13);
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

test("an open shot editor immediately shows a newly attached private previs", () => {
  const previs = { assetId: "previs", purpose: "camera", start: 0, end: 5 };
  const p = fixture();
  const base = p.nodes[1].references!;
  p.previsAssets = [asset("previs", "video")];
  p.nodes[1].references = [...base, previs];
  const visible = mergeReferenceDraft(base, base, p.nodes[1].references);
  expect(visible).toEqual([...base, previs]);
  const edited = visible.map((r) =>
    r.assetId === "previs" ? { ...r, purpose: "movement" } : r,
  );
  const saved = editReferences(p, "shot", p.nodes[1].references, edited);
  expect(saved.nodes[1].references).toEqual([
    ...base,
    { ...previs, purpose: "movement" },
  ]);
  expect(saved.assets.some((a) => a.id === "previs")).toBe(false);
});

test("reference refresh preserves unsaved edits and removals, and accepts remote changes", () => {
  const base = [
    { assetId: "product", purpose: "appearance" },
    { assetId: "remove", purpose: "old" },
    { assetId: "remote", purpose: "before" },
  ];
  const local = [
    { assetId: "product", purpose: "unsaved purpose" },
    base[2],
    { assetId: "local", purpose: "new local reference" },
  ];
  const previs = { assetId: "previs", purpose: "camera", start: 0, end: 5 };
  const current = [
    base[0],
    base[1],
    { ...base[2], purpose: "updated" },
    previs,
  ];
  const visible = mergeReferenceDraft(base, local, current);
  expect(visible).toEqual([
    local[0],
    { ...base[2], purpose: "updated" },
    previs,
    local[2],
  ]);
  // Once visible, a new host reference can also be removed in the same editor.
  const removedPrevis = visible.filter((r) => r.assetId !== "previs");
  expect(mergeReferenceDraft(current, removedPrevis, current)).toEqual(
    removedPrevis,
  );
  // An unchanged reference deleted remotely must not reappear.
  expect(mergeReferenceDraft(base, base, [])).toEqual([]);
});
