import { expect, test } from "bun:test";
import { asset, fixture } from "../production/fixtures.test-helper";
import {
  canUseProductionReference,
  isPlanningMaterial,
  isShotAsset,
} from "./resourceRoles";
import {
  collectAsset,
  uncollectAsset,
  setAssetUsage,
  unconfirmStoryAsset,
} from "./assetLibrary";
import { inspectProject } from "../assistant/inspectProject";
import { addStoryAsset } from "./storyAssetLibrary";
import { applyOperations } from "../assistant/projectCommands";
import { requestTask, runsOf } from "../production/requestTask";

function setup() {
  const p = fixture();
  p.assets.push(
    asset("source"),
    { ...asset("hero"), usage: "asset" },
    { ...asset("candidate"), usage: "asset", generated: true },
  );
  p.nodes[1].references = [];
  return p;
}
const context = (p: ReturnType<typeof fixture>) => ({
  callId: "boundary",
  turnId: "turn",
  turn: { projectId: p.id, models: {}, instruction: "Produce" },
});

test("research, confirmed assets, candidates and shot outputs have distinct roles", () => {
  const p = setup();
  const byId = (id: string) => p.assets.find((a) => a.id === id)!;
  expect(isPlanningMaterial(byId("source"))).toBe(true);
  expect(isShotAsset(byId("source"))).toBe(false);
  expect(canUseProductionReference(p, byId("source"))).toBe(false);
  expect(canUseProductionReference(p, byId("hero"))).toBe(true);
  expect(canUseProductionReference(p, byId("candidate"))).toBe(false);
  expect(canUseProductionReference(p, byId("a"))).toBe(true); // Saved storyboard output.
  expect(canUseProductionReference(p, byId("v"))).toBe(true); // Saved shot take.
  expect(isPlanningMaterial(byId("a"), p)).toBe(false);
  expect(isPlanningMaterial(byId("v"), p)).toBe(false);
  const confirmed = addStoryAsset(p, "candidate");
  expect(isShotAsset(confirmed.assets.find((a) => a.id === "candidate")!)).toBe(
    true,
  );
  expect(
    isPlanningMaterial(confirmed.assets.find((a) => a.id === "candidate")!),
  ).toBe(false);
});

test("explicit source IDs cannot bypass the shot-generation boundary; asset creation can use sources", () => {
  const p = setup();
  const op = {
    mediaKind: "image",
    id: "shot",
    text: "Storyboard",
    references: [{ assetId: "source", purpose: "product" }],
  };
  expect(() => requestTask(p, op, context(p))).toThrow(
    "此资源尚未确认为镜头资产",
  );
  const created = requestTask(
    p,
    { ...op, id: undefined, generationPurpose: "asset" },
    context(p),
  );
  expect(runsOf(created)[0]).toMatchObject({
    generationPurpose: "asset",
    status: "AWAITING_CONFIRMATION",
    inputs: [{ assetId: "source" }],
  });
  expect(() =>
    requestTask(
      p,
      { ...op, references: [{ assetId: "candidate", purpose: "identity" }] },
      context(p),
    ),
  ).toThrow();
});

test("only confirmed assets can be linked to shots, including edits of existing references", () => {
  const p = setup();
  const link = (id: string) => [
    {
      op: "set_references",
      id: "shot",
      references: [{ assetId: id, purpose: "identity" }],
    },
  ];
  for (const id of ["source", "candidate", "a"])
    expect(() => applyOperations(p, link(id))).toThrow(
      "镜头只能关联已确认资产",
    );
  p.nodes[1].references = [{ assetId: "source", purpose: "old" }];
  expect(() => applyOperations(p, link("source"))).toThrow(
    "镜头只能关联已确认资产",
  );
  const next = applyOperations(p, link("hero"));
  expect(next.nodes[1].references).toEqual([
    { assetId: "hero", purpose: "identity" },
  ]);
  expect(next.assets).toBe(p.assets);
});

test("one source remains readable in both libraries and memberships can be removed independently", () => {
  const original = setup();
  const next = addStoryAsset(original, "source");
  const read = (p: typeof next, usage: string) =>
    (
      inspectProject(p, { section: "assets", usage }).items as { id: string }[]
    ).map((a) => a.id);
  expect(read(next, "material")).toContain("source");
  expect(read(next, "asset")).toContain("source");
  expect(next.assets).toHaveLength(original.assets.length);
  expect(next.assets.find((a) => a.id === "source")?.path).toBe(
    original.assets.find((a) => a.id === "source")?.path,
  );
  const noMaterial = uncollectAsset(next, "source");
  expect(read(noMaterial, "material")).not.toContain("source");
  expect(read(noMaterial, "asset")).toContain("source");
  expect(
    canUseProductionReference(
      noMaterial,
      noMaterial.assets.find((a) => a.id === "source")!,
    ),
  ).toBe(true);
  const pending = unconfirmStoryAsset(next, "source");
  expect(read(pending, "material")).toContain("source");
  expect(read(pending, "asset")).not.toContain("source");
  expect(pending.assets.find((a) => a.id === "source")?.assetStatus).toBe(
    "candidate",
  );
  const noAsset = setAssetUsage(next, "source", "material");
  expect(read(noAsset, "material")).toContain("source");
  expect(read(noAsset, "asset")).not.toContain("source");
});

test("collecting an asset candidate as material does not approve it for shots", () => {
  let p = setup();
  const candidate = p.assets.find((a) => a.id === "candidate")!;
  p = collectAsset(p, candidate);
  const collected = p.assets.find((a) => a.id === candidate.id)!;
  expect(isPlanningMaterial(collected, p)).toBe(true);
  expect(isShotAsset(collected)).toBe(false);
  expect(canUseProductionReference(p, collected)).toBe(false);
  expect(() =>
    applyOperations(p, [
      {
        op: "set_references",
        id: "shot",
        references: [{ assetId: candidate.id, purpose: "identity" }],
      },
    ]),
  ).toThrow();
  p = addStoryAsset(p, candidate.id);
  const confirmed = p.assets.find((a) => a.id === candidate.id)!;
  expect(isPlanningMaterial(confirmed, p)).toBe(true);
  expect(isShotAsset(confirmed)).toBe(true);
  expect(
    applyOperations(p, [
      {
        op: "set_references",
        id: "shot",
        references: [{ assetId: candidate.id, purpose: "identity" }],
      },
    ]).nodes[1].references,
  ).toEqual([{ assetId: candidate.id, purpose: "identity" }]);
});
