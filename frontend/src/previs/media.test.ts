import { expect, test } from "bun:test";
import { newProject, type Asset } from "../model";
import { isLibraryAsset, collectAsset } from "../workspace/assetLibrary";
import {
  replaceTaskReferences,
  assertGenerationReferences,
} from "../production/taskReferences";
import { requestPrompt, registeredInput } from "../creation/generationInput";
import { inspectProject } from "../assistant/inspectProject";
import { referenceAsset } from "./media";
import type { ProductionTask } from "../production/types";
import { model } from "../production/fixtures.test-helper";

const video: Asset = {
  id: "private-preview",
  name: "Previs",
  path: "/preview.mp4",
  preview: "",
  kind: "video",
  duration: 3,
  width: 720,
  height: 1280,
  hasAudio: false,
  generated: true,
};
test("private video resolves for generation and exact reads while library and inventories stay separate", () => {
  const project = { ...newProject("Private refs"), previsAssets: [video] };
  const task: ProductionTask = {
    key: "t",
    kind: "video",
    mode: "multi",
    inputs: [],
    prompt: "Follow action",
    modelId: "m",
  };
  const next = replaceTaskReferences(project, task, [
    { assetId: video.id, purpose: "Camera motion", start: 0, end: 3 },
  ]);
  expect(() => assertGenerationReferences(project, next)).not.toThrow();
  expect(next.inputs[0].assetId).toBe(video.id);
  expect(referenceAsset(project, video.id)).toEqual(video);
  expect(
    inspectProject(project, { section: "assets", ids: [video.id] }).items,
  ).toHaveLength(1);
  expect(inspectProject(project, { section: "assets" }).items).toHaveLength(0);
  expect(project.assets.filter(isLibraryAsset)).toHaveLength(0);
  expect(requestPrompt(project, task.prompt, next.inputs)).toContain(
    "Video 1: Camera motion",
  );
  const mediaModel = model("minimax/h3/reference-to-video");
  expect(() =>
    registeredInput(mediaModel, project, task.prompt, next.inputs, [
      {
        assetId: video.id,
        kind: "video",
        role: "reference",
        url: "https://fal.media/previs.mp4",
      },
    ]),
  ).not.toThrow();
});
test("collecting an ordinary generated result changes membership without altering its frames or tasks", () => {
  const image = { ...video, id: "frame", kind: "image" as const };
  const project = { ...newProject("Frames"), assets: [image] };
  expect(project.assets.filter(isLibraryAsset)).toHaveLength(0);
  const saved = collectAsset(project, image);
  expect(saved.assets.filter(isLibraryAsset)).toHaveLength(1);
  expect(saved.nodes).toBe(project.nodes);
  expect(saved.production).toBe(project.production);
  expect(saved.assets[0].path).toBe(image.path);
});
