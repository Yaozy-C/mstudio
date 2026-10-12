import { expect, test } from "bun:test";
import { fixture, asset } from "./fixtures.test-helper";
import { requestTask, runsOf } from "./requestTask";
import { editTask, canEditOriginal } from "./taskEditing";
import { receiveProductionResult } from "./document";

test("saving and revising a draft keeps prompt and inputs together, without automatic dispatch", () => {
  const p = fixture();
  p.nodes[1].shot!.framePrompt = "legacy prompt";
  const context = {
    callId: "save",
    turnId: "turn",
    turn: {
      projectId: p.id,
      models: { image: "model", execution: "automatic" as const },
      instruction: "draft",
    },
  };
  const op = {
    id: "shot",
    mediaKind: "image",
    text: "new prompt",
    draftOnly: true,
    references: [{ assetId: "b", role: "reference", purpose: "product" }],
  };
  const saved = requestTask(p, op, context);
  const first = runsOf(saved)[0];
  expect(first.status).toBe("DRAFT");
  expect(canEditOriginal(first)).toBe(true);
  expect(saved.nodes).toEqual(p.nodes);
  const generated = requestTask(
    saved,
    { id: "shot", mediaKind: "image", text: "new prompt" },
    { ...context, callId: "generate-with-refs" },
  );
  expect(runsOf(generated)[0].key).toBe(first.key);
  expect(runsOf(generated)[0].inputs).toEqual(first.inputs);
  expect(runsOf(generated)[0].status).toBe("READY");

  const revised = requestTask(
    saved,
    { ...op, text: "corrected prompt", references: [] },
    { ...context, callId: "revise" },
  );
  expect(runsOf(revised)).toHaveLength(1);
  expect(runsOf(revised)[0].key).toBe(first.key);
  expect(runsOf(revised)[0].inputs).toEqual([]);
  const submitted = requestTask(
    revised,
    { id: "shot", mediaKind: "image", text: "corrected prompt" },
    { ...context, callId: "generate" },
  );
  expect(runsOf(submitted)).toHaveLength(1);
  expect(runsOf(submitted)[0].status).toBe("READY");
  // Explicitly cleared inputs must not silently inherit the shot's old references.
  expect(runsOf(submitted)[0].inputs).toEqual([]);
  expect(submitted.nodes[1].shot!.framePrompt).toBe("legacy prompt");
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
    { id: "shot", mediaKind: "image", text: "original", draftOnly: true },
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
  const edited = editTask(saved, task.key, {
    text: "changed",
    references: [{ assetId: "b", purpose: "product" }],
  });
  expect(runsOf(edited)[0].inputs.map((r) => r.assetId)).toEqual(["b"]);
  expect(runsOf(edited)[0].status).toBe("DRAFT");
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
    { mediaKind: "image", text: "only on task", references: [] },
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
