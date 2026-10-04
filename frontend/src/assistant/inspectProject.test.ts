import { expect, test } from "bun:test";
import { newProject } from "../model";
import { applyOperations, inspectProject } from "./projectCommands";

test("a six-shot handoff is read once without unrelated video prompts", () => {
  const initial = newProject("Task handoff");
  const p = applyOperations(initial, [
    { op: "add_node", id: "screenplay", kind: "screenplay", title: "Plan" },
    ...Array.from({ length: 6 }, (_, i) => ({
      op: "add_node",
      id: `shot${i}`,
      kind: "shot",
      title: `Shot ${i}`,
      text: "lift",
      shot: {
        screenplayId: "screenplay",
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
  const p = applyOperations(newProject("Compact"), [
    { op: "add_node", id: "screenplay", kind: "screenplay", title: "Plan" },
    ...Array.from({ length: 6 }, (_, i) => ({
      op: "add_node",
      id: `s${i}`,
      kind: "shot",
      title: "Shot",
      text: "lift bag",
      shot: {
        screenplayId: "screenplay",
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
  const p = applyOperations(newProject("Local edit"), [
    { op: "set_brief", text: "Unrelated project brief" },
    { op: "add_node", id: "screenplay", kind: "screenplay", title: "Plan" },
    ...Array.from({ length: 12 }, (_, i) => ({
      op: "add_node",
      id: `s${i}`,
      kind: "shot",
      title: "Shot",
      text,
      shot: {
        screenplayId: "screenplay",
        order: i + 1,
        duration: 2,
        dialogue: "",
        prompt: "提示词".repeat(800),
      },
    })),
  ]);
  const first = inspectProject(p, {
    nodeIds: ["s0"],
    fields: ["prompt", "text", "shot"],
  });
  expect(first.details).toHaveLength(1);
  expect(first.brief).toBeUndefined();
  expect(first.tracks).toEqual([]);
  expect(first.nextOffset).toBeNull();
  expect(first.details?.[0].shot?.screenplayId).toBe("screenplay");
  expect(first.details?.[0].shot?.prompt?.nextTextOffset).toBeNull();
  const next = first.details?.[0].nextTextOffset;
  expect(next).toBe(4000);
  const second = inspectProject(p, {
    nodeIds: ["s0"],
    fields: ["text"],
    textOffset: next,
  });
  expect(first.details![0].text! + second.details![0].text!).toBe(text);
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

test("shot ordering reads exclude long text and scripts, with explicit fields", () => {
  const p = applyOperations(newProject("Fields"), [
    {
      op: "add_node",
      id: "screenplay",
      kind: "screenplay",
      title: "Plan",
      screenplay: {
        script: [
          {
            id: "p1",
            title: "Script",
            action: "unrelated".repeat(600),
            duration: 2,
            dialogue: "",
            sound: "",
          },
        ],
      },
    },
    {
      op: "add_node",
      id: "shot",
      kind: "shot",
      title: "Thermal",
      text: "long action".repeat(1000),
      shot: {
        screenplayId: "screenplay",
        order: 1,
        duration: 2,
        dialogue: "long speech".repeat(400),
      },
    },
  ]);
  const result = inspectProject(p, {
    nodeIds: ["shot"],
    fields: ["title", "shot.order", "shot.duration"],
  });
  expect(result.details?.[0].title).toBe("Thermal");
  expect(result.details?.[0].shot?.order).toBe(1);
  expect(result.details?.[0].shot?.duration).toBe(2);
  expect(result.details?.[0].text).toBeUndefined();
  expect(result.details?.[0].shot?.dialogue).toBeUndefined();
  expect(JSON.stringify(result).length).toBeLessThan(1000);
  const screenplay = inspectProject(p, {
    nodeIds: ["screenplay"],
    fields: ["shots"],
  });
  expect(screenplay.details?.[0].shots).toEqual([
    { id: "shot", title: "Thermal", order: 1, duration: 2 },
  ]);
  expect(screenplay.details?.[0].screenplay).toBeUndefined();
  expect(
    inspectProject(p, { nodeIds: ["shot"] }).details?.[0].text,
  ).toBeUndefined();
});

test("duration edit reads only its paragraph, reports saved values, and preserves every other field", async () => {
  const { savedValues } = await import("./savedValues");
  let p = applyOperations(newProject("Eight shots"), [
    {
      op: "add_node",
      id: "screenplay",
      kind: "screenplay",
      title: "Script",
      screenplay: {
        script: Array.from({ length: 8 }, (_, i) => ({
          id: `p${i + 1}`,
          title: `Paragraph ${i + 1}`,
          duration: 2,
          action: "Long action ".repeat(100),
          dialogue: "Speech",
          sound: "Sound",
        })),
      },
    },
    ...Array.from({ length: 8 }, (_, i) => ({
      op: "add_node",
      id: `s${i + 1}`,
      kind: "shot",
      title: "Shot",
      text: "Preserve action",
      shot: {
        screenplayId: "screenplay",
        scriptId: `p${i + 1}`,
        order: i + 1,
        duration: 2,
        dialogue: "Preserve speech",
      },
    })),
  ]);
  const before = structuredClone(p);
  const read = {
    nodeIds: ["screenplay"],
    fields: ["script"],
    paragraphIds: ["p4"],
    scriptFields: ["duration"],
  };
  const detail = inspectProject(p, read).details![0].screenplay!;
  expect(detail.script).toEqual([{ id: "p4", duration: 2 }]);
  expect(detail.omittedScriptFields).toContain("action");
  expect(detail.missingParagraphIds).toEqual([]);
  expect(
    inspectProject(p, { ...read, paragraphIds: ["absent"] }).details![0]
      .screenplay!.missingParagraphIds,
  ).toEqual(["absent"]);
  const ops = [
    {
      op: "update_node",
      id: "screenplay",
      screenplay: { script: [{ id: "p4", duration: 1.5 }] },
    },
    { op: "update_node", id: "s4", shot: { duration: 1.5 } },
  ];
  p = applyOperations(p, ops);
  expect(savedValues(p, ops)).toMatchObject([
    {
      id: "screenplay",
      exists: true,
      complete: true,
      values: { screenplay: { script: p.nodes[0].screenplay!.script } },
    },
    {
      id: "s4",
      exists: true,
      complete: true,
      values: { shot: { duration: 1.5 } },
    },
  ]);
  expect(inspectProject(p, read).details![0].screenplay!.script).toEqual([
    { id: "p4", duration: 1.5 },
  ]);
  p.nodes[0].screenplay!.script![3].duration = 2;
  p.nodes.find((n) => n.id === "s4")!.shot!.duration = 2;
  expect(p.nodes.find((n) => n.id === "s4")!.shot!.visualChanged).toBe(true);
  delete p.nodes.find((n) => n.id === "s4")!.shot!.visualChanged;
  expect(p).toEqual(before);
  const summary = JSON.stringify(
    inspectProject(p, { nodeIds: ["screenplay"] }),
  );
  expect(summary).not.toContain("Long action");
  const full = inspectProject(p, {
    nodeIds: ["screenplay"],
    fields: ["screenplay"],
    paragraphIds: ["p4"],
  });
  expect(JSON.stringify(full)).toContain("Long action");
});
