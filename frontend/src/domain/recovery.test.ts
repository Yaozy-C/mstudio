import { expect, test } from "bun:test";
import { asset, fixture } from "../production/fixtures.test-helper";
import { saveTask } from "../production/document";
import type { ProductionTask } from "../production/types";
import type { GeneratedJob } from "../workspace/generatedJobTracking";
import { recoverJobs } from "./recovery";

test("batch recovery isolates a bad job, preserves deleted outputs and is idempotent", () => {
  const task = (key: string): ProductionTask => ({
    key,
    kind: "image",
    mode: "multi",
    prompt: "Keep",
    inputs: [],
    modelId: "image",
    jobId: key,
    submissionId: key,
    status: "IN_QUEUE",
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
  const result = recoverJobs(before, [bad, job("good"), job("removed")]);
  expect(result.failedJobs).toBe(1);
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
