import { expect, test } from "bun:test";
import { clipLabel } from "./clipLabel";
import { fixture } from "../production/fixtures.test-helper";

test("generated clip labels use actual shot ownership while imported names stay verbatim", () => {
  const project = fixture();
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
  expect(clipLabel(project, clip, asset)).toBe("Pack");
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
  expect(clipLabel(project, clip, asset)).toBe("视频 01");
  expect(asset.name).toBe("生成结果-123.mp4");
});
