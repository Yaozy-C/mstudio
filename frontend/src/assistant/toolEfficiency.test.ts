import { expect, test } from "bun:test";
import { newProject } from "../model";
import { inspectProject } from "./inspectProject";
import { clipReceipt } from "./savedValues";
import { defaultGrade } from "../timeline/grading";

function fixture(count = 8) {
  const p = newProject("Batch editing");
  p.clips = Array.from({ length: count }, (_, i) => ({
    id: `c${i}`,
    assetId: `a${i}`,
    start: i * 2,
    trimIn: 0,
    trimOut: 2,
    speed: 1,
    volume: 1,
    trackId: "v1",
    visual: {
      brightness: 0,
      contrast: 1,
      saturation: 1,
      temperature: 0,
      effect: "none" as const,
      grade: defaultGrade(),
    },
  }));
  return p;
}

test("eight graded clips fit one bounded read, while large projects paginate without gaps", () => {
  const small = inspectProject(fixture(), { section: "clips" });
  expect(small.items).toHaveLength(8);
  expect(small.nextOffset).toBeNull();
  const p = fixture(30);
  let offset = 0;
  const ids: string[] = [];
  do {
    const page = inspectProject(p, { section: "clips", offset });
    expect(page.items!.length).toBeLessThanOrEqual(12);
    expect(
      new TextEncoder().encode(JSON.stringify(page.items)).length,
    ).toBeLessThan(12500);
    ids.push(...(page.items as { id: string }[]).map((c) => c.id));
    if (page.nextOffset === null) break;
    expect(page.nextOffset).toBeGreaterThan(offset);
    offset = page.nextOffset!;
  } while (offset < 30);
  expect(ids).toEqual(p.clips.map((c) => c.id));
});

test("receipts expose actual saved deltas, removals and ripple neighbours without unchanged fields", () => {
  const before = fixture();
  const after = structuredClone(before);
  after.clips[0].speed = 2;
  after.clips[1].start = 1;
  after.clips[2].visual!.grade!.exposure = 0.4;
  after.clips.pop();
  const receipt = clipReceipt(before, after);
  expect(receipt.complete).toBe(true);
  expect(receipt.total).toBe(4);
  expect(receipt.items[0]).toEqual({
    id: "c0",
    exists: true,
    values: { speed: 2 },
  });
  expect(receipt.items[1].values).toEqual({ start: 1 });
  expect(receipt.items[2].values.visual).toEqual(after.clips[2].visual!);
  expect(receipt.items[3].exists).toBe(false);
  expect(clipReceipt(before, before)).toEqual({
    items: [],
    total: 0,
    complete: true,
  });
  expect(clipReceipt(newProject("empty"), fixture(20)).complete).toBe(false);
});
