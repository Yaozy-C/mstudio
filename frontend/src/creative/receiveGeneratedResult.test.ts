import { expect, test } from "bun:test";
import { newProject, type Asset, type BoardNode } from "../model";
import { receiveGeneratedResult } from "./receiveGeneratedResult";
import { createTask } from "../production/tasks";
import { productionItems } from "../production/items";
import { canvasSnapshot } from "../production/request";
test("results target the submitted shot and a generated image can immediately become video input", () => {
  const p = newProject("results");
  const shot: BoardNode = {
    id: "s",
    kind: "shot",
    title: "shot",
    text: "",
    x: 0,
    y: 0,
    shot: { planId: "p", order: 0, duration: 5, dialogue: "" },
  };
  p.nodes = [shot];
  const image: Asset = {
    id: "i",
    kind: "image",
    path: "/i",
    preview: "/i",
    name: "image",
    duration: 0,
    width: 10,
    height: 10,
    hasAudio: false,
  };
  const source = {
    ...shot,
    canvasGeneration: canvasSnapshot(
      p,
      { ...createTask(p, productionItems(p)), ownerId: "s" },
      {
        x: 0,
        y: 0,
      },
    ),
  };
  const result = receiveGeneratedResult(p, image, source);
  expect(result.nodes[0].shot?.frames?.[0]?.assetId).toBe("i");
  expect(receiveGeneratedResult(result, image, source)).toBe(result);
  const video: Asset = { ...image, id: "v", kind: "video", duration: 5 };
  const videoSource = {
    ...result.nodes[0],
    canvasGeneration: canvasSnapshot(
      result,
      {
        ...createTask(
          result,
          productionItems(result).filter((n) => n.kind === "image"),
          "video",
        ),
        ownerId: "s",
      },
      { x: 0, y: 0 },
    ),
  };
  const withVideo = receiveGeneratedResult(result, video, videoSource);
  expect(withVideo.nodes[0].shot?.takes?.at(-1)?.assetId).toBe("v");
  expect(withVideo.nodes[0].resultAssetId).toBeUndefined();
  expect(withVideo.clips).toEqual(p.clips);
  expect(receiveGeneratedResult(withVideo, video, videoSource)).toBe(withVideo);
});
