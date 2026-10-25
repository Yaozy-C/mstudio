import { expect, test } from "bun:test";
import { imageReviewController } from "./imageReviewController";
import type { Asset, Project } from "../../model";
import type { CreativeTask } from "../../creative/aiTasks";

const original = {
  id: "original",
  kind: "image",
  name: "original.png",
} as Asset;
const marked = { id: "marked", kind: "image", name: "comments.png" } as Asset;
const comments = [
  { id: "one", x: 0, y: 0, width: 0.2, height: 0.2, text: "Lower the handle" },
];
const file = new File(["test"], "comments.png", { type: "image/png" });

test("changing the selected file behind a stable asset ID rejects stale annotations", async () => {
  const source = { ...original, path: "/original.png" };
  let project = { id: "p", assets: [source] } as Project;
  let submitted = false;
  const controller = imageReviewController(
    () => project,
    (fn) => {
      project = fn(project);
    },
    { items: [] },
    {
      importFile: async () => {
        project = {
          ...project,
          assets: [{ ...source, path: "/v1.png" }],
        };
        return marked;
      },
      request: () => {
        submitted = true;
      },
    },
  );
  await expect(controller.submit(source, file, comments)).rejects.toThrow(
    "参考版本已变更",
  );
  expect(submitted).toBe(false);
});

test("submitting a review attaches only the source and keeps the guide internal", async () => {
  let project = { id: "p", assets: [original] } as Project;
  let request: CreativeTask | undefined;
  const controller = imageReviewController(
    () => project,
    (update) => {
      project = update(project);
    },
    { items: [] },
    {
      importFile: async () => marked,
      request: (task) => {
        request = task;
      },
    },
  );
  await controller.submit(original, file, comments);
  expect(project.assets).toEqual([
    original,
    { ...marked, annotationSourceId: original.id },
  ]);
  expect(request?.refs).toEqual([{ kind: "asset", id: "original" }]);
  expect(request?.agentId).toBe("image");
});

test("removing the original during upload prevents a stale edit request", async () => {
  let project = { id: "p", assets: [original] } as Project;
  let submitted = false;
  const controller = imageReviewController(
    () => project,
    (update) => {
      project = update(project);
    },
    { items: [] },
    {
      importFile: async () => {
        project = { ...project, assets: [] };
        return marked;
      },
      request: () => {
        submitted = true;
      },
    },
  );
  await expect(controller.submit(original, file, comments)).rejects.toThrow();
  expect(submitted).toBe(false);
});

test("full attachment drafts are rejected before importing a new annotation image", async () => {
  const project = { id: "p", assets: [original] } as Project;
  let imported = false;
  const controller = imageReviewController(
    () => project,
    () => {},
    {
      items: Array.from({ length: 12 }, (_, i) => ({
        kind: "asset",
        id: `existing-${i}`,
      })),
    },
    {
      importFile: async () => {
        imported = true;
        return marked;
      },
      request: () => {},
    },
  );
  await expect(controller.submit(original, file, comments)).rejects.toThrow();
  expect(imported).toBe(false);
});

test("a review can update an already attached image when the draft has 12 items", async () => {
  let project = { id: "p", assets: [original] } as Project;
  let request: CreativeTask | undefined;
  const controller = imageReviewController(
    () => project,
    (update) => {
      project = update(project);
    },
    {
      items: [
        { kind: "asset", id: "original" },
        ...Array.from({ length: 11 }, (_, i) => ({
          kind: "asset" as const,
          id: `existing-${i}`,
        })),
      ],
    },
    {
      importFile: async () => marked,
      request: (task) => {
        request = task;
      },
    },
  );
  await controller.submit(original, file, comments);
  expect(request?.refs).toEqual([{ kind: "asset", id: "original" }]);
});
