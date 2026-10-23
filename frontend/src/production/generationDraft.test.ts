import { expect, test } from "bun:test";
import { fixture, asset } from "./fixtures.test-helper";
import { requestTask, runsOf } from "./requestTask";
import { editTask, canEditOriginal } from "./taskEditing";
import { receiveProductionResult } from "./document";

test("pending task edits preserve inputs and always await user confirmation", () => {
  const p = fixture();
  p.nodes[1].shot!.framePrompt = "stored shot draft";
  const saved = requestTask(
    p,
    {
      id: "shot",
      title: "Pack · 分镜图",
      mediaKind: "image",
      text: "new prompt",
      references: [{ assetId: "b", role: "reference", purpose: "product" }],
    },
    {
      callId: "create",
      turnId: "turn",
      turn: {
        projectId: p.id,
        models: { image: "model", execution: "automatic" as const },
        instruction: "prepare image",
      },
    },
  );
  const first = runsOf(saved)[0];
  expect(first.status).toBe("AWAITING_CONFIRMATION");
  expect(canEditOriginal(first)).toBe(true);
  expect(saved.nodes).toEqual(p.nodes);
  const revised = editTask(saved, first.key, { text: "corrected prompt" });
  expect(runsOf(revised)).toHaveLength(1);
  expect(runsOf(revised)[0].inputs).toEqual(first.inputs);
  const cleared = editTask(revised, first.key, { references: [] });
  expect(runsOf(cleared)[0].inputs).toEqual([]);
  expect(runsOf(cleared)[0].prompt).toBe("corrected prompt");
  expect(runsOf(cleared)[0].status).toBe("AWAITING_CONFIRMATION");
  expect(cleared.nodes[1].shot!.framePrompt).toBe("stored shot draft");
});

test("invalid reference changes cannot partially replace a saved prompt", () => {
  const p = fixture();
  const context = {
    callId: "save",
    turnId: "turn",
    turn: { projectId: p.id, models: {}, instruction: "draft" },
  };
  const saved = requestTask(
    p,
    {
      id: "shot",
      title: "Pack · 分镜图",
      mediaKind: "image",
      text: "original",
    },
    context,
  );
  const task = runsOf(saved)[0];
  expect(() =>
    editTask(saved, task.key, {
      text: "changed",
      references: [{ assetId: "missing", purpose: "product" }],
    }),
  ).toThrow();
  expect(runsOf(saved)[0].prompt).toBe("original");
  for (const purpose of ["x".repeat(401), 42])
    expect(() =>
      editTask(saved, task.key, { references: [{ assetId: "b", purpose }] }),
    ).toThrow();
  const edited = editTask(saved, task.key, {
    text: "changed",
    references: [{ assetId: "b", purpose: "x".repeat(400) }],
  });
  expect(runsOf(edited)[0].inputs.map((r) => r.assetId)).toEqual(["b"]);
  expect(runsOf(edited)[0].inputs[0].purpose).toHaveLength(400);
  expect(runsOf(edited)[0].status).toBe("AWAITING_CONFIRMATION");
});

test("new result cards do not duplicate generation prompts", () => {
  const p = fixture();
  const context = {
    callId: "generate",
    turnId: "turn",
    turn: { projectId: p.id, models: {}, instruction: "generate" },
  };
  const saved = requestTask(
    p,
    {
      mediaKind: "image",
      title: "人物资产",
      text: "only on task",
      references: [],
    },
    context,
  );
  const task = runsOf(saved)[0];
  const next = receiveProductionResult(saved, asset("output"), {
    id: "source",
    kind: "asset",
    title: "source",
    text: "",
    x: 0,
    y: 0,
    canvasGeneration: { task, x: 0, y: 0 },
  });
  expect(next.nodes.find((n) => n.assetId === "output")!.text).toBe("");
  expect(runsOf(next)[0].prompt).toBe("only on task");
});
