import { expect, test } from "bun:test";
import { fixture } from "../production/fixtures.test-helper";
import { deleteAssets, deleteLegacyHiddenAssets } from "./deleteAssets";
import { productionItems } from "../production/items";
import { createTask } from "../production/tasks";
import { inspectProject } from "../assistant/inspectProject";
import { reconcileJob } from "../domain/generation";
import { needsTracking } from "./generatedJobTracking";
import { restoreProject } from "./restoreProject";
import "../domain/entry";

test("permanent deletion clears every live use and agent discovery, without changing other media", () => {
  const p = fixture();
  p.nodes.push({
    id: "card",
    kind: "asset",
    assetId: "a",
    title: "a",
    text: "",
    x: 0,
    y: 0,
  });
  p.nodes[1].references!.push({ assetId: "a", purpose: "reference" });
  p.clips.push({
    id: "clip",
    assetId: "a",
    trimIn: 0,
    trimOut: 3,
    speed: 1,
    volume: 1,
    start: 0,
    trackId: "v1",
  });
  p.captions.push({
    id: "caption",
    assetId: "a",
    start: 0,
    end: 3,
    text: "derived",
  });
  const task = {
    ...createTask(p, [], "image"),
    key: "run",
    inputs: [
      {
        key: "asset:a",
        assetId: "a",
        role: "reference" as const,
        purpose: "product",
      },
    ],
    status: "READY",
    resultAssetId: "a",
    resultAssetIds: ["a", "b"],
  };
  p.production = {
    drafts: { run: task },
    hidden: ["node:card"],
    positions: { "node:card": { x: 1, y: 2 } },
  };
  const next = deleteAssets(p, ["a"]);
  expect(p.assets).toHaveLength(4);
  expect(next.assets.map((a) => a.id)).toEqual(["b", "reference", "v"]);
  expect(next.nodes.some((n) => n.id === "card")).toBe(false);
  expect(next.nodes[1].shot?.frames?.map((f) => f.assetId)).toEqual(["b"]);
  expect(next.nodes[1].references).toEqual([
    { assetId: "reference", purpose: "product" },
  ]);
  expect(next.clips).toEqual([]);
  expect(next.captions).toEqual([]);
  expect(next.production?.drafts?.run).toMatchObject({
    inputs: [],
    status: "FAILED",
    resultAssetIds: ["b"],
  });
  expect(next.production?.drafts?.run.resultAssetId).toBeUndefined();
  expect(productionItems(next).some((i) => i.assetId === "a")).toBe(false);
  expect(
    JSON.stringify(inspectProject(next, { section: "assets", ids: ["a"] })),
  ).not.toContain('"id":"a"');
  expect(restoreProject(p, next).assets).toEqual(next.assets);
});

test("legacy hidden cards become deletions while visible accepted versions survive", () => {
  const p = fixture();
  p.nodes.push({
    id: "wrong",
    kind: "asset",
    assetId: "a",
    title: "wrong",
    text: "",
    x: 0,
    y: 0,
  });
  p.production = { hidden: ["node:wrong", "take:shot:v"] };
  const next = deleteLegacyHiddenAssets(p);
  expect(next.assets.map((a) => a.id)).toEqual(["b", "reference"]);
  expect(next.removedAssetIds).toEqual(["a", "v"]);
  expect(deleteLegacyHiddenAssets(next)).toBe(next);
});

test("late job reconciliation does not resurrect a deleted output or keep tracking it", () => {
  const p = fixture();
  const task = {
    ...createTask(p, [], "image"),
    key: "run",
    turnId: "turn",
    jobId: "job",
    status: "COMPLETED",
    resultAssetId: "a",
    resultAssetIds: ["a"],
  };
  p.production = { drafts: { run: task } };
  const job = {
    id: "job",
    status: "COMPLETED",
    outputCount: 1,
    assets: [p.assets[0]],
    shot: { ...p.nodes[1], canvasGeneration: { task, x: 0, y: 0 } },
  };
  const next = reconcileJob(deleteAssets(p, ["a"]), job);
  expect(next.assets.some((a) => a.id === "a")).toBe(false);
  expect(next.production?.drafts?.run.resultAssetIds).toEqual([]);
  expect(next.production?.drafts?.run.status).toBe("COMPLETED");
  expect(needsTracking(job, next)).toBe(false);
});

test("concurrent edits cannot restore removed references through merge", () => {
  const p = fixture();
  const local = structuredClone(p);
  local.nodes[1].shot!.frames![0].prompt = "stale edit";
  const current = deleteAssets(p, ["a"]);
  const execute = (
    globalThis as unknown as { domainExecute(raw: string): string }
  ).domainExecute;
  const result = JSON.parse(
    execute(
      JSON.stringify({ action: "merge", base: p, document: local, current }),
    ),
  );
  expect(result.error).toBeUndefined();
  expect(result.document.assets.some((a: { id: string }) => a.id === "a")).toBe(
    false,
  );
  expect(
    result.document.nodes[1].shot.frames.map(
      (f: { assetId: string }) => f.assetId,
    ),
  ).toEqual(["b"]);
});
