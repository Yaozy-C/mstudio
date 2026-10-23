import { expect, test } from "bun:test";
import { clipLabel } from "./clipLabel";
import { fixture } from "../production/fixtures.test-helper";
import { createTask } from "../production/tasks";
import { saveTask } from "../production/document";

test("generated clips use their named task or explicitly assigned shot, never user instructions or guessed numbers", () => {
  let project = fixture();
  const asset = {
    ...project.assets[3],
    generated: true,
    name: "生成结果-123.mp4",
  };
  project.assets[3] = asset;
  const clip = {
    id: "c",
    assetId: asset.id,
    trimIn: 0,
    trimOut: 5,
    speed: 1,
    volume: 1,
    start: 0,
    trackId: "v1",
  };
  const task = {
    ...createTask(project, [], "video"),
    title: "Pack · video",
    resultAssetId: asset.id,
    instruction: "Generate everything",
  };
  project = saveTask(project, task);
  expect(clipLabel(project, clip, asset)).toBe("Pack · video");
  expect(clipLabel(project, clip, { ...asset, generated: false })).toBe(
    asset.name,
  );
  project.nodes.push({
    ...project.nodes[1],
    id: "shared",
    title: "Another shot",
  });
  expect(clipLabel(project, { ...clip, shotId: "shared" }, asset)).toBe(
    "Another shot",
  );
  expect(() => saveTask(project, { ...task, title: undefined })).toThrow(
    "任务名称",
  );
  expect(asset.name).toBe("生成结果-123.mp4");
});
