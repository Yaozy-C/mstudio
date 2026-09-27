import { expect, test } from "bun:test";
import { newProject } from "../model";
import { applyOperations, inspectProject } from "./projectCommands";

test("a six-shot handoff is read once without unrelated video prompts", () => {
  const initial = newProject("Task handoff");
  const p = applyOperations(initial, 0, [
    { op: "add_node", id: "plan", kind: "plan", title: "Plan" },
    ...Array.from({ length: 6 }, (_, i) => ({
      op: "add_node",
      id: `shot${i}`,
      kind: "shot",
      title: `Shot ${i}`,
      text: "lift",
      shot: {
        planId: "plan",
        order: i + 1,
        duration: 2,
        dialogue: "",
        framePrompt: "image",
        prompt: "irrelevant video".repeat(100),
      },
    })),
  ]);
  const result = inspectProject(p, {
    nodeIds: Array.from({ length: 6 }, (_, i) => `shot${i}`),
    fields: ["framePrompt"],
  });
  expect(result.details).toHaveLength(6);
  expect(result.details?.[0].shot?.framePrompt?.text).toBe("image");
  expect(result.details?.[0].shot?.prompt).toBeUndefined();
  expect(result.details?.[0].shot?.frames).toBeUndefined();
});

test("default batch details omit long prompts and can include creation", () => {
  const p = applyOperations(newProject("Compact"), 0, [
    { op: "add_node", id: "plan", kind: "plan", title: "Plan" },
    ...Array.from({ length: 6 }, (_, i) => ({
      op: "add_node",
      id: `s${i}`,
      kind: "shot",
      title: "Shot",
      text: "lift bag",
      shot: {
        planId: "plan",
        order: i + 1,
        duration: 2,
        dialogue: "",
        framePrompt: "长图片描述".repeat(1000),
        prompt: "长视频描述".repeat(1000),
      },
    })),
  ]);
  const result = inspectProject(p, {
    nodeIds: Array.from({ length: 6 }, (_, i) => `s${i}`),
    section: "creation",
  });
  expect(result.details).toHaveLength(6);
  expect(result.details?.[0].shot?.framePrompt).toBeUndefined();
  expect(result.details?.[0].shot?.prompt).toBeUndefined();
  expect(JSON.stringify(result).length).toBeLessThan(16000);
});

test("targeted reads keep local context without unrelated pagination and can finish long text", () => {
  const text = "镜头动作".repeat(1250);
  const p = applyOperations(newProject("Local edit"), 0, [
    { op: "set_brief", text: "Unrelated project brief" },
    { op: "add_node", id: "plan", kind: "plan", title: "Plan" },
    ...Array.from({ length: 12 }, (_, i) => ({
      op: "add_node",
      id: `s${i}`,
      kind: "shot",
      title: "Shot",
      text,
      shot: {
        planId: "plan",
        order: i + 1,
        duration: 2,
        dialogue: "",
        prompt: "提示词".repeat(800),
      },
    })),
  ]);
  const first = inspectProject(p, { nodeIds: ["s0"], fields: ["prompt"] });
  expect(first.details).toHaveLength(1);
  expect(first.brief).toBeUndefined();
  expect(first.tracks).toEqual([]);
  expect(first.nextOffset).toBeNull();
  expect(first.details?.[0].shot?.planId).toBe("plan");
  expect(first.details?.[0].shot?.prompt?.nextTextOffset).toBeNull();
  const next = first.details?.[0].nextTextOffset;
  expect(next).toBe(4000);
  const second = inspectProject(p, { nodeIds: ["s0"], textOffset: next });
  expect(first.details![0].text + second.details![0].text).toBe(text);
  expect(second.details?.[0].nextTextOffset).toBeNull();
  const overview = inspectProject(p, {});
  expect(overview.nextOffset).toBe(10);
  expect(overview.brief).toBe("Unrelated project brief");
});

test("exact clip reads exclude other targets and expose omitted fields", () => {
  const p = newProject("Scoped reading");
  p.clips = ["a", "b"].map((id) => ({
    id,
    assetId: `asset-${id}`,
    trimIn: 1,
    trimOut: 4,
    speed: 1,
    volume: 0.8,
    start: 0,
    trackId: "v",
    x: 25,
  }));
  const result = inspectProject(p, {
    section: "clips",
    ids: ["b", "missing"],
    fields: ["trimIn", "trimOut", "speed"],
  });
  expect(result.items).toEqual([
    {
      id: "b",
      trimIn: 1,
      trimOut: 4,
      speed: 1,
      omittedFields: ["assetId", "volume", "start", "trackId", "x"],
    },
  ]);
  expect(result.missingIds).toEqual(["missing"]);
  const full = inspectProject(p, { section: "clips", ids: ["b"] });
  expect(full.items).toEqual([p.clips[1]]);
});
