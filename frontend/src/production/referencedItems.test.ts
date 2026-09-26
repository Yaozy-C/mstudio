import { expect, test } from "bun:test";
import { fixture } from "./fixtures.test-helper";
import { productionItems } from "./items";
import { referencedItems } from "./referencedItems";
import { createTask } from "./tasks";

test("reference borders follow the composer inputs, including removal", () => {
  const p = fixture();
  const items = productionItems(p);
  const frame = items.find((item) => item.assetId === "a")!;
  const task = createTask(p, [frame]);
  expect(referencedItems(p, items, task, []).has(frame.key)).toBe(true);
  task.inputs = task.inputs.filter((input) => input.key !== frame.key);
  const referenced = referencedItems(p, items, task, []);
  expect(referenced.has(frame.key)).toBe(false);
  expect(
    referenced.has(items.find((item) => item.kind === "script")!.key),
  ).toBe(false);
  expect(
    referenced.has(items.find((item) => item.assetId === "reference")!.key),
  ).toBe(false);
});

test("a canvas reference marks its card rather than unrelated copies", () => {
  const p = fixture();
  const items = productionItems(p);
  const frame = items.find((item) => item.assetId === "a")!;
  const copy = { ...frame, key: "another-shot-frame", ownerId: "another-shot" };
  const task = createTask(p, [frame]);
  const referenced = referencedItems(p, [...items, copy], task, []);
  expect(referenced.has(frame.key)).toBe(true);
  expect(referenced.has(copy.key)).toBe(false);
});

test("@ references resolve asset and text cards without canvas selection", () => {
  const p = fixture();
  p.nodes.push({
    id: "note",
    kind: "note",
    title: "Brief",
    text: "Keep the bag blue",
    x: 0,
    y: 0,
  });
  const items = productionItems(p);
  const frame = items.find((item) => item.assetId === "a")!;
  const note = items.find((item) => item.nodeId === "note")!;
  expect(
    referencedItems(p, items, undefined, [
      { kind: "asset", id: "a" },
      { kind: "node", id: "note" },
    ]),
  ).toEqual(new Set([frame.key, note.key]));
  expect(referencedItems(p, items, undefined, [])).toEqual(new Set());
});

test("an active canvas task excludes attachments outside the active composer", () => {
  const p = fixture();
  const items = productionItems(p);
  const frame = items.find((item) => item.assetId === "a")!;
  const other = items.find((item) => item.assetId === "b")!;
  const task = createTask(p, [frame]);
  expect(
    referencedItems(p, items, task, [{ kind: "asset", id: "b" }]).has(
      other.key,
    ),
  ).toBe(false);
  task.inputs = [];
  expect(referencedItems(p, items, task, [{ kind: "asset", id: "b" }])).toEqual(
    new Set(),
  );
});
