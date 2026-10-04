import { expect, test } from "bun:test";
import { newProject } from "../model";
import { savedValues } from "../assistant/savedValues";
import { inspectProject } from "../assistant/inspectProject";
import { generationOutcomes, taskOutcome } from "../production/taskOutcome";
import type { ProductionTask } from "../production/types";
import { receipt } from "./receipt";

const task = (status: string): ProductionTask => ({
  key: "run:execution:0",
  kind: "image",
  mode: "single",
  inputs: [],
  modelId: "model",
  prompt: "Kitchen",
  turnId: "turn",
  createdAt: 1,
  status,
});

test("node receipts return committed prompts, frames, references and cleared fields", () => {
  const p = newProject("receipts");
  p.nodes = [
    {
      id: "shot",
      kind: "shot",
      title: "Saved",
      text: "Actual action",
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      references: [{ assetId: "product", purpose: "identity" }],
      shot: {
        order: 1,
        duration: 7,
        screenplayId: "script",
        dialogue: "",
        framePrompt: "Actual saved prompt",
        frames: [{ assetId: "result", title: "Frame" }],
      },
    },
  ];
  const operations = [
    {
      op: "update_node",
      id: "shot",
      text: "Not saved",
      shot: {
        framePrompt: "Requested value is not proof",
        prompt: null,
        frames: [],
      },
      references: [],
    },
  ];
  expect(savedValues(p, operations)).toEqual([
    {
      id: "shot",
      exists: true,
      complete: true,
      values: {
        text: "Actual action",
        references: p.nodes[0].references,
        shot: {
          framePrompt: "Actual saved prompt",
          prompt: null,
          frames: p.nodes[0].shot!.frames,
        },
      },
    },
  ]);
});

test("created task and explicit status reads provide the same actionable continuation", () => {
  const before = newProject("generation");
  const after = structuredClone(before);
  const pending = task("AWAITING_CONFIRMATION");
  after.production = { drafts: { [pending.key]: pending } };
  const result = receipt(before, after, [], "execution");
  expect(result.generationTasks[0]).toMatchObject({
    status: "AWAITING_CONFIRMATION",
    resultAssetIds: [],
    continuation: {
      state: "waiting_user",
      action: "confirm_generation",
      trigger: "user_confirmation",
    },
  });
  const read = inspectProject(after, {
    section: "generation",
    taskKey: pending.key,
    fields: ["status"],
  });
  expect(read.items![0]).toMatchObject({
    continuation: result.generationTasks[0].continuation,
  });
});

test("backend processing, import, pause, failure and real result have distinct continuations", () => {
  for (const status of ["READY", "IN_PROGRESS", "RECEIVING", "COMPLETED"])
    expect(taskOutcome(task(status)).continuation.state).toBe(
      "waiting_service",
    );
  expect(
    taskOutcome({ ...task("COMPLETED"), resultAssetIds: ["real"] }),
  ).toMatchObject({
    resultAssetIds: ["real"],
    continuation: { state: "ready", action: "inspect_result" },
  });
  expect(
    taskOutcome({ ...task("RECEIVING"), trackingPaused: true }).continuation
      .action,
  ).toBe("resume_tracking");
  expect(taskOutcome(task("UNKNOWN")).continuation.action).toBe(
    "resolve_unknown_submission",
  );
  for (const status of ["FAILED", "CANCELLED"])
    expect(taskOutcome(task(status)).continuation.state).toBe("finished");
  expect(
    generationOutcomes(newProject("removed"), ["absent"])[0],
  ).toMatchObject({
    status: "REMOVED",
    continuation: { state: "finished" },
  });
});
