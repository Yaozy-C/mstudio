import { test, expect } from "bun:test";
import { newProject } from "../model";
import { newModel, newShot, isStale, fileIdentity } from "./types";
test("asset revision invalidates only shots using that shared model", () => {
  const model = { ...newModel("Lunch bag"), revision: 1 };
  const project = newProject("Test");
  const shot = {
    ...newShot(
      {
        id: "shot",
        kind: "shot",
        title: "Shot",
        text: "",
        x: 0,
        y: 0,
        shot: { screenplayId: "s", order: 1, duration: 3, dialogue: "" },
      },
      project,
    ),
    revision: 2,
    assetIds: [model.id],
  };
  const video = {
    id: "v",
    fileId: "shot",
    fileRevision: 2,
    modelRevisions: { [model.id]: 1 },
    signature: "sig",
    asset: {} as never,
    savedAssetId: null,
  };
  expect(isStale(video, [model, shot])).toBe(false);
  expect(isStale(video, [{ ...model, revision: 2 }, shot])).toBe(true);
  expect(fileIdentity(shot, [model])).not.toBe(
    fileIdentity(shot, [{ ...model, revision: 2 }]),
  );
  expect(fileIdentity(shot, [model, newModel("Unrelated")])).toBe(
    fileIdentity(shot, [model]),
  );
});
test("new shots use exact shot timing without model minimum generation duration", () => {
  const project = newProject("Test");
  const shot = newShot(
    {
      id: "s",
      kind: "shot",
      title: "Insert",
      text: "Bottle cap",
      x: 0,
      y: 0,
      shot: { screenplayId: "p", order: 1, duration: 0.4, dialogue: "" },
    },
    project,
  );
  expect(shot.duration).toBe(0.4);
  expect(shot.description).toBe("Bottle cap");
  expect(shot.code).toBe("");
  expect(shot.assetIds).toEqual([]);
});
