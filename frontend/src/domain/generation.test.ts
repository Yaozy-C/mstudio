import { expect, test } from "bun:test";
import { asset, fixture, model } from "../production/fixtures.test-helper";
import { prepareGeneration, reconcileJob } from "./generation";
import { saveTask } from "../production/document";
import type { ProductionTask } from "../production/types";
import type { GeneratedJob } from "../workspace/generatedJobTracking";
import { parameterContract } from "../production/parameters";
test("headless preflight and model capabilities share video parameter limits", () => {
  const video = model("minimax/h3/reference-to-video");
  const task: ProductionTask = {
    key: "run",
    kind: "video",
    mode: "multi",
    prompt: "Walk forward",
    inputs: [],
    modelId: video.id,
    parameters: { duration: 7, resolution: "480P" },
    submissionId: "submission",
  };
  const project = saveTask(fixture(), task);
  const request = prepareGeneration(project, "run", video, []);
  expect(request.input.duration).toBe(7);
  expect(request.input.resolution).toBe("480P");
  expect(parameterContract(video).parameters).toMatchObject({
    properties: { duration: { minimum: 5, maximum: 15 } },
  });
  expect(() =>
    prepareGeneration(
      saveTask(project, { ...task, parameters: { duration: 3 } }),
      "run",
      video,
    ),
  ).toThrow();
});
test("partial outputs attach once and background errors retain received media", () => {
  const task: ProductionTask = {
    key: "run",
    kind: "image",
    mode: "multi",
    prompt: "Portrait",
    inputs: [],
    modelId: "image",
    jobId: "job",
    submissionId: "job",
    turnId: "turn",
    status: "IN_QUEUE",
  };
  const source = {
    ...fixture().nodes[1],
    canvasGeneration: { task, x: 0, y: 0 },
  };
  const job: GeneratedJob = {
    id: "job",
    status: "COMPLETED",
    shot: source,
    outputCount: 2,
    assets: [asset("one")],
  };
  let p = reconcileJob(saveTask(fixture(), task), job);
  expect(p.production?.drafts?.run.status).toBe("RECEIVING");
  expect(p.production?.drafts?.run.resultAssetIds).toEqual(["one"]);
  expect(reconcileJob(p, job)).toEqual(p);
  const removed = {
    ...p,
    nodes: p.nodes.filter((n) => n.assetId !== "one"),
    assets: p.assets.filter((a) => a.id !== "one"),
  };
  expect(reconcileJob(removed, job)).toEqual(removed);
  job.assets!.push(asset("two"));
  job.sync = { stage: "import", error: "connection interrupted", paused: true };
  p = reconcileJob(p, job);
  expect(p.production?.drafts?.run.status).toBe("COMPLETED");
  expect(p.production?.drafts?.run.trackingPaused).toBe(true);
  expect(p.production?.drafts?.run.resultAssetIds).toEqual(["one", "two"]);
  expect(reconcileJob(p, job)).toEqual(p);
});
