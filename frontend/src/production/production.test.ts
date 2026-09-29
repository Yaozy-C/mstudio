import { expect, test } from "bun:test";
import { fixture, asset, model } from "./fixtures.test-helper";
import { framesOf } from "./frames";
import { productionItems } from "./items";
import { createTask, withMode } from "./tasks";
import { canvasSnapshot, inputFor } from "./request";
import { receiveGeneratedResult } from "../creative/receiveGeneratedResult";
import { saveTask, jobStatus } from "./document";
import { restoreProject } from "../workspace/restoreProject";
import type { ProductionSource } from "./types";
test("selected frames can generate video directly, including after script and prompt edits", () => {
  const p = fixture();
  const task = createTask(
    p,
    productionItems(p).filter((n) => n.kind === "image"),
    "video",
  );
  expect(
    canvasSnapshot(p, task, { x: 0, y: 0 }).task.inputs.filter(
      (r) => r.assetId,
    ),
  ).toHaveLength(2);
  p.nodes[0].screenplay!.script![0].action = "Close bag";
  p.nodes[1].shot!.frames![0].prompt = "Closer";
  expect(() => canvasSnapshot(p, task, { x: 0, y: 0 })).not.toThrow();
  p.nodes[1].shot!.frames = [];
  expect(() => canvasSnapshot(p, task, { x: 0, y: 0 })).not.toThrow();
});
test("first/last roles map exactly; unsupported model or incomplete roles fail before upload", () => {
  let p = fixture();
  const items = productionItems(p).filter((n) => n.kind === "image");
  const task = {
    ...withMode(createTask(p, items, "video"), "ends"),
    prompt: "Close the lid",
  };
  task.inputs[0].role = "first-frame";
  task.inputs[1].role = "last-frame";
  expect(canvasSnapshot(p, task, { x: 0, y: 0 }).task).toEqual(task);
  const m = model("minimax/h3/image-to-video");
  m.params = {
    image_url: "https://hidden.invalid/input",
    end_image_url: "https://hidden.invalid/end",
    duration: 5,
  };
  const result = inputFor(p, task, m, [
    { assetId: "a", kind: "image", url: "https://example.test/a" },
    { assetId: "b", kind: "image", url: "https://example.test/b" },
  ]);
  expect(result.image_url).toBe("https://example.test/a");
  expect(result.end_image_url).toBe("https://example.test/b");
  expect(JSON.stringify(result)).not.toContain("hidden.invalid");
  expect(() =>
    inputFor(p, task, model("minimax/h3-max/image-to-video")),
  ).toThrow("不支持");
  expect(() =>
    inputFor(p, { ...task, inputs: task.inputs.slice(0, 1) }, m),
  ).toThrow("首尾帧");
});
test("mixed references preserve intervals and never claim video editing support", () => {
  const p = fixture();
  const task = {
    ...createTask(
      p,
      productionItems(p).filter(
        (n) => n.kind === "image" || n.key.startsWith("take:"),
      ),
      "video",
    ),
    prompt: "Keep the action",
  };
  expect(task.mode).toBe("mixed");
  const m = model("minimax/h3/reference-to-video");
  const input = inputFor(p, task, m);
  expect(input.reference_image_urls).toHaveLength(2);
  expect(input.reference_video_urls).toHaveLength(1);
  const invalid = {
    ...task,
    inputs: task.inputs.map((r) => (r.assetId === "v" ? { ...r, end: 18 } : r)),
  };
  expect(() => inputFor(p, invalid, m)).toThrow("2–15 秒");
  expect(() =>
    inputFor(
      p,
      {
        ...task,
        inputs: task.inputs.map((r) =>
          r.assetId === "v" ? { ...r, role: "video-edit" as const } : r,
        ),
      },
      m,
    ),
  ).toThrow("暂不支持");
});
test("result uses the frozen task; importing twice is idempotent and preserves originals, timeline and other drafts", () => {
  let p = fixture();
  const originals = structuredClone(p);
  const task = {
    ...createTask(
      p,
      productionItems(p)
        .filter((n) => n.kind === "image")
        .slice(0, 1),
    ),
    ownerId: "shot",
    prompt: "Closer",
    modelId: "m",
    jobId: "job",
  };
  const snapshot = canvasSnapshot(p, task, { x: 400, y: 110 });
  const source: ProductionSource = {
    ...p.nodes[1],
    canvasGeneration: snapshot,
  };
  p = saveTask(p, task);
  p = saveTask(p, {
    ...task,
    key: "another",
    prompt: "Different",
    jobId: "other",
  });
  const result = asset("new");
  p = receiveGeneratedResult(p, result, source);
  expect(framesOf(p.nodes[1]).map((f) => f.assetId)).toEqual(["a", "b", "new"]);
  expect(receiveGeneratedResult(p, result, source)).toBe(p);
  expect(p.nodes[1].shot!.frames![0].assetId).toBe("a");
  expect(p.nodes[1].shot!.takes).toEqual(originals.nodes[1].shot!.takes);
  expect(p.clips).toEqual(originals.clips);
  expect(p.production!.drafts!.another.prompt).toBe("Different");
  const next = jobStatus(p, "job", "COMPLETED");
  expect(next.production!.drafts!.another.status).toBeUndefined();
  expect(next.production!.drafts![task.key].status).toBe("COMPLETED");
  const undone = restoreProject(originals, p);
  expect(framesOf(undone.nodes[1])).toHaveLength(2);
  expect(undone.assets.some((a) => a.id === "new")).toBe(true);
  const gone = {
    ...originals,
    nodes: originals.nodes.filter((n) => n.id !== "shot"),
  };
  const orphan = receiveGeneratedResult(gone, result, source);
  expect(orphan.nodes.some((n) => n.assetId === "new")).toBe(true);
});
test("video results retain exact media roles and undo never rewinds paid job status", () => {
  const p = fixture();
  const task = {
    ...withMode(
      createTask(
        p,
        productionItems(p).filter((n) => n.kind === "image"),
        "video",
      ),
      "ends",
    ),
    ownerId: "shot",
    prompt: "Close",
    turnId: "turn",
    createdAt: 1,
    jobId: "paid-job",
    status: "IN_QUEUE",
  };
  const source: ProductionSource = {
    ...p.nodes[1],
    canvasGeneration: canvasSnapshot(p, task, { x: 900, y: 100 }),
  };
  const changed = saveTask(p, {
    ...task,
    prompt: "Edited while generating",
    inputs: task.inputs.slice(0, 1),
  });
  const result = receiveGeneratedResult(
    changed,
    asset("result", "video"),
    source,
  );
  expect(result.nodes[1].shot!.takes!.at(-1)!.production!.inputs).toHaveLength(
    2,
  );
  expect(result.nodes[1].shot!.takes!.at(-1)!.production!.prompt).toBe("Close");
  const completed = jobStatus(result, "paid-job", "COMPLETED");
  const undone = restoreProject(changed, completed);
  expect(undone.nodes[1].shot!.takes).toHaveLength(1);
  expect(undone.production!.drafts![task.key].status).toBe("COMPLETED");
});
