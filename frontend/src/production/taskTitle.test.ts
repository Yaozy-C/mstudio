import { expect, test } from "bun:test";
import { fixture, asset } from "./fixtures.test-helper";
import { requestTask, runsOf } from "./requestTask";
import { taskTitle } from "./taskTitle";
import { receiveProductionResult, saveTask } from "./document";
import { applyOperations, inspectProject } from "../assistant/projectCommands";
import { regenerationDraft } from "./taskEditing";
import { generationOperations } from "../domain/generationContract";
import { createTask } from "./tasks";
import { resultTaskName } from "./taskResultNames";

const context = (projectId: string, callId = "call") => ({
  callId,
  turnId: "turn",
  turn: { projectId, models: {}, instruction: "生成资产图吧" },
});

test("one generic user request creates independently named asset, storyboard and video tasks", () => {
  let p = fixture();
  const specs = [
    {
      title: "小满 · 人物资产",
      mediaKind: "image",
      generationPurpose: "asset",
    },
    {
      title: "海滩 · 黄昏场景",
      mediaKind: "image",
      generationPurpose: "asset",
    },
    { title: "Pack · 分镜图", mediaKind: "image", id: "shot" },
    { title: "Pack · 视频", mediaKind: "video", id: "shot" },
  ];
  for (const [i, spec] of specs.entries())
    p = requestTask(
      p,
      { ...spec, text: `Prompt ${i}`, references: [] },
      context(p.id, `call-${i}`),
    );
  const tasks = runsOf(p);
  expect(tasks.map((task) => taskTitle(task))).toEqual(
    specs.map((spec) => spec.title),
  );
  expect(tasks.every((task) => task.instruction === "生成资产图吧")).toBe(true);
  expect(tasks.every((task) => task.status === "AWAITING_CONFIRMATION")).toBe(
    true,
  );
  const read = inspectProject(p, { section: "generation", fields: ["title"] });
  expect(JSON.stringify(read)).toContain("小满 · 人物资产");
  for (const operation of generationOperations)
    expect(operation.required).toContain("title");
});

test("unnamed tasks never infer names from shots, instructions, prompts or result filenames", () => {
  const p = fixture();
  const task = {
    ...createTask(p, []),
    targetNodeId: "shot",
    instruction: "生成资产图吧",
    prompt: "Long production prompt",
    resultAssetId: "a",
  };
  expect(() => taskTitle(task)).toThrow("任务名称");
  expect(taskTitle({ ...task, title: "小满 · 人物资产" })).toBe(
    "小满 · 人物资产",
  );
});

test("every Agent generation branch requires a bounded explicit name, including inherited drafts", () => {
  const p = fixture();
  for (const spec of [
    { mediaKind: "image", id: "shot" },
    { mediaKind: "video", id: "shot" },
    { mediaKind: "image", generationPurpose: "asset" },
  ])
    for (const title of [undefined, " ", 42, "x".repeat(101)])
      expect(() =>
        requestTask(
          p,
          { ...spec, text: "Prompt", references: [], title },
          {
            ...context(p.id),
            turn: {
              ...context(p.id).turn,
              task: { ...createTask(p, []), title: "Old draft name" },
            },
          },
        ),
      ).toThrow("任务名称");
});

test("asset names are required, bounded, and become result display names without renaming files", () => {
  const p = fixture();
  const op = {
    mediaKind: "image",
    generationPurpose: "asset",
    text: "Person",
    references: [],
  };
  for (const title of [undefined, " ", 42, "x".repeat(101)])
    expect(() => requestTask(p, { ...op, title }, context(p.id))).toThrow(
      "任务名称",
    );
  let next = requestTask(
    p,
    { ...op, title: " 小满 · 人物资产 " },
    context(p.id),
  );
  const task = runsOf(next)[0];
  const result = { ...asset("result"), name: "生成结果-123.png" };
  next = receiveProductionResult(next, result, {
    id: "source",
    kind: "asset",
    title: "Source",
    text: "",
    x: 0,
    y: 0,
    canvasGeneration: { task, x: 0, y: 0 },
  });
  expect(next.assets.find((a) => a.id === result.id)).toMatchObject({
    name: "小满 · 人物资产",
    path: result.path,
    id: result.id,
  });
  expect(next.nodes.find((n) => n.assetId === result.id)?.title).toBe(
    "小满 · 人物资产",
  );
  expect(result.name).toBe("生成结果-123.png");
});

test("title-only edits preserve active job identity, prompt, references and generation state", () => {
  let p = fixture();
  const original = runsOf(
    requestTask(
      p,
      {
        title: "Pack · 分镜图",
        mediaKind: "image",
        id: "shot",
        text: "Complete prompt",
        references: [],
      },
      context(p.id),
    ),
  )[0];
  for (const status of ["READY", "IN_PROGRESS", "COMPLETED"])
    for (const title of ["Pack · 分镜图修订", "Pack · 视频候选"]) {
      const task = {
        ...original,
        status,
        jobId: "job",
        submissionId: "submission",
      };
      p = saveTask(p, task);
      const next = applyOperations(p, [
        { op: "update_generation", taskKey: task.key, title },
      ]);
      expect(next.production!.drafts![task.key]).toEqual({ ...task, title });
      expect(next.assets).toBe(p.assets);
      expect(
        regenerationDraft({ ...task, status: "COMPLETED", title }).title,
      ).toBe(title);
    }
});

test("renaming a task updates its result display names and preserves physical media paths", () => {
  const p = fixture();
  const task = {
    ...createTask(p, []),
    title: "Original",
    resultAssetIds: ["a"],
  };
  p.nodes.push({
    id: "card",
    kind: "asset",
    assetId: "a",
    title: "Original",
    text: "",
    x: 0,
    y: 0,
  });
  const next = applyOperations(saveTask(p, task), [
    { op: "update_generation", taskKey: task.key, title: "Renamed" },
  ]);
  expect(next.assets[0]).toEqual({ ...p.assets[0], name: "Renamed" });
  expect(next.nodes.find((node) => node.id === "card")?.title).toBe("Renamed");
  expect(next.nodes[1]).toBe(p.nodes[1]);
  expect(next.assets[1]).toBe(p.assets[1]);
});

test("result naming uses the current task identity and rejects a missing required name", () => {
  const p = fixture();
  const snapshot = { ...createTask(p, []), title: "Name at submission" };
  const source = {
    ...p.nodes[1],
    canvasGeneration: { task: snapshot, x: 0, y: 0 },
  };
  expect(resultTaskName(p, source)).toBe("Name at submission");
  const renamed = saveTask(p, { ...snapshot, title: "Current name" });
  expect(resultTaskName(renamed, source)).toBe("Current name");
  const unnamed = saveTask(p, { ...snapshot, title: undefined });
  expect(() => resultTaskName(unnamed, source)).toThrow("任务名称");
});
