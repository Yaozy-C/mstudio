import { expect, test } from "bun:test";
import { asset, fixture } from "../production/fixtures.test-helper";
import { saveTask } from "../production/document";
import type { ProductionTask } from "../production/types";
import type { GeneratedJob } from "../workspace/generatedJobTracking";
import { recoverJobs } from "./recovery";

test("batch recovery isolates a bad job, preserves deleted outputs and is idempotent", () => {
  const task = (key: string): ProductionTask => ({
    key,
    title: `Frame ${key}`,
    kind: "image",
    mode: "multi",
    prompt: "Keep",
    inputs: [],
    modelId: "image",
    jobId: key,
    submissionId: key,
    status: "IN_QUEUE",
    trackingPaused: true,
  });
  const job = (key: string): GeneratedJob => ({
    id: key,
    status: "COMPLETED",
    outputCount: 1,
    assets: [asset(key)],
    shot: {
      ...fixture().nodes[1],
      canvasGeneration: { task: task(key), x: 0, y: 0 },
    },
  });
  let before = fixture();
  for (const key of ["bad", "good", "removed"])
    before = saveTask(before, task(key));
  before.removedAssetIds = ["removed"];
  const original = structuredClone(before);
  const bad = job("bad");
  bad.assets = [asset("wrong-kind", "video")];
  bad.sync = {
    stage: "reconcile",
    attempt: 4,
    paused: true,
    error: "prior failure",
  };
  const good = job("good");
  good.sync = {
    stage: "reconcile",
    attempt: 4,
    paused: true,
    error: "old failure",
  };
  const result = recoverJobs(before, [bad, good, job("removed")]);
  expect(result.failures).toHaveLength(1);
  expect(result.failures[0].jobId).toBe("bad");
  expect(JSON.parse(result.failures[0].error)).toMatchObject({
    code: "PROJECT_SYNC_FAILED",
    stage: "reconcile",
    retryable: true,
  });
  expect(result.document.production!.drafts!.bad.error).toBe(
    result.failures[0].error,
  );
  expect(result.document.production!.drafts!.bad.trackingPaused).toBe(true);
  expect(result.document.production!.drafts!.good.trackingPaused).toBe(false);
  expect(result.document.production!.drafts!.good.error).toBeUndefined();
  expect(bad.sync.attempt).toBe(4);
  expect(bad.sync.paused).toBe(true);
  expect(before).toEqual(original);
  expect(result.document.production!.drafts!.bad.status).toBe("IN_QUEUE");
  expect(result.document.production!.drafts!.good.status).toBe("COMPLETED");
  expect(result.document.assets.some((a) => a.id === "good")).toBe(true);
  expect(result.document.assets.some((a) => a.id === "removed")).toBe(false);
  expect(result.document.revision).toBe((before.revision ?? 0) + 1);
  expect(
    recoverJobs(result.document, [bad, job("good"), job("removed")]),
  ).toEqual(result);
});
