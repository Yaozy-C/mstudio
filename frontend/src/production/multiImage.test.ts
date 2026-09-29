import { expect, test } from "bun:test";
import { fixture, asset } from "./fixtures.test-helper";
import { requestTask, runsOf } from "./requestTask";
import { receiveProductionResult, saveTask } from "./document";
import { retryTask } from "./recovery";
import { productionItems } from "./items";
import {
  needsTracking,
  type GeneratedJob,
} from "../workspace/generatedJobTracking";
import type { ProductionSource } from "./types";

function setup() {
  let p = fixture();
  p = requestTask(
    p,
    {
      mediaKind: "image",
      text: "依次生成 FRONT、RIGHT、TOP 三张独立图片",
    },
    {
      callId: "multi",
      turnId: "turn",
      turn: {
        projectId: p.id,
        models: { image: "codex" },
        instruction: "裁剪三视图",
      },
    },
  );
  const task = { ...runsOf(p)[0], jobId: "job" };
  p = saveTask(p, task);
  const source = { canvasGeneration: { task, x: 0, y: 0 } } as ProductionSource;
  return { p, task, source };
}
test("all tool outputs survive repeated imports", () => {
  let { p, source } = setup();
  for (const id of ["front", "right", "top", "front", "right"])
    p = receiveProductionResult(p, asset(id), source);
  const run = runsOf(p)[0];
  expect(run.resultAssetIds).toEqual(["front", "right", "top"]);
  expect(run.resultAssetId).toBe("front");
  expect(
    p.nodes.filter((n) => ["front", "right", "top"].includes(n.assetId ?? "")),
  ).toHaveLength(3);
});
test("partial failure retains its output without rewriting a retry request", () => {
  let { p, task, source } = setup();
  const job = {
    id: "job",
    status: "FAILED",
    outputCount: 1,
    shot: source,
  } as GeneratedJob;
  task = { ...task, status: "FAILED" };
  p = saveTask(p, task);
  expect(needsTracking(job, p)).toBe(true);
  p = receiveProductionResult(p, asset("front"), source);
  expect(needsTracking(job, p)).toBe(false);
  const retry = retryTask(runsOf(p)[0]);
  expect(retry.prompt).toBe(task.prompt);
  expect(retry.resultAssetIds).toBeUndefined();
  expect(retry.jobId).toBeUndefined();
  expect(runsOf(p)[0].resultAssetIds).toEqual(["front"]);
});
test("all outputs must be imported before tracking stops, including after an import failure", () => {
  let { p, source } = setup();
  const job = {
    id: "job",
    status: "COMPLETED",
    outputCount: 3,
    shot: source,
  } as GeneratedJob;
  p = receiveProductionResult(p, asset("front"), source);
  p = saveTask(p, { ...runsOf(p)[0], status: "COMPLETED" });
  expect(needsTracking(job, p)).toBe(true);
  p = receiveProductionResult(p, asset("right"), source);
  p = receiveProductionResult(p, asset("top"), source);
  expect(needsTracking(job, p)).toBe(false);
});

test("multiple outputs sharing a source position do not move earlier cards or overlap", () => {
  let { p, source } = setup();
  source.canvasGeneration!.task.position = { x: -1200, y: 87 };
  p = receiveProductionResult(p, asset("front"), source);
  const first = productionItems(p).find((n) => n.assetId === "front")!;
  const originalPosition = { x: first.x, y: first.y };
  p = receiveProductionResult(p, asset("right"), source);
  p = receiveProductionResult(p, asset("top"), source);
  const results = productionItems(p).filter((n) =>
    ["front", "right", "top"].includes(n.assetId ?? ""),
  );
  expect(source.canvasGeneration!.task.position).toEqual({ x: -1200, y: 87 });
  expect({ x: results[0].x, y: results[0].y }).toEqual(originalPosition);
  expect(new Set(results.map((n) => `${n.x},${n.y}`)).size).toBe(3);
  for (let i = 0; i < results.length; i++)
    for (let j = i + 1; j < results.length; j++) {
      const a = results[i],
        b = results[j];
      expect(
        a.x < b.x + b.width &&
          a.x + a.width > b.x &&
          a.y < b.y + b.height &&
          a.y + a.height > b.y,
      ).toBe(false);
    }
});

test("late output from a previous attempt cannot overwrite a retried task", () => {
  let { p, task, source } = setup();
  const retry = retryTask({ ...task, status: "FAILED" });
  p = saveTask(p, { ...retry, jobId: "new-job", status: "IN_PROGRESS" });
  const received = receiveProductionResult(p, asset("old-output"), source);
  expect(received.production!.drafts![task.key].jobId).toBe("new-job");
  expect(received.production!.drafts![task.key].resultAssetIds).toBeUndefined();
  expect(received.assets.some((a) => a.id === "old-output")).toBe(true);
  expect(Object.keys(received.production!.drafts!)).toEqual(
    Object.keys(p.production!.drafts!),
  );
});
