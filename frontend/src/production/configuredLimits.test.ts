import { expect, test } from "bun:test";
import { model, fixture, asset } from "./fixtures.test-helper";
import { inputFor } from "./request";
import { createTask } from "./tasks";
import { parameterContract, taskParameters } from "./parameters";
import { validateCapabilities } from "../models/capabilities";

test("even a known endpoint uses configuration alone for numeric controls", () => {
  const m = model("minimax/h3/image-to-video", "video", {
    controls: {
      duration: { path: "/custom/seconds", min: 0.25, max: 120 },
      resolution: { path: "/custom/resolution", values: ["8k-custom"] },
      aspectRatio: { path: "/custom/ratio", values: ["7:1"] },
    },
  });
  validateCapabilities(m.capabilities);
  expect(
    taskParameters(m, {
      duration: 60.5,
      resolution: "8k-custom",
      aspectRatio: "7:1",
    }),
  ).toEqual({
    custom: { seconds: 60.5, resolution: "8k-custom", ratio: "7:1" },
  });
  expect(() => taskParameters(m, { duration: 121 })).toThrow();
  m.capabilities!.controls!.duration = { path: "/seconds" };
  expect(taskParameters(m, { duration: 180.25 }).seconds).toBe(180.25);
  const properties = parameterContract(m).parameters.properties as Record<
    string,
    Record<string, unknown>
  >;
  expect(properties.duration.maximum).toBeUndefined();
});

test("reference count and clip length have no hidden limits beyond configuration", () => {
  const p = fixture();
  p.assets.push(...Array.from({ length: 16 }, (_, i) => asset(`extra-${i}`)));
  const m = model("custom/model", "video", {
    references: [
      { key: "/images", kind: "image", role: "reference", multiple: true },
      { key: "/videos", kind: "video", role: "reference", multiple: true },
    ],
  });
  const task = {
    ...createTask(p, [], "video"),
    prompt: "Follow these references",
    inputs: [
      ...p.assets
        .filter((a) => a.id.startsWith("extra-"))
        .map((a) => ({
          key: a.id,
          assetId: a.id,
          role: "reference" as const,
          purpose: a.name,
        })),
      {
        key: "v",
        assetId: "v",
        role: "video-reference" as const,
        purpose: "motion",
        start: 0,
        end: 20,
      },
    ],
  };
  const result = inputFor(p, task, m);
  expect(result.images).toHaveLength(16);
  expect(result.videos).toHaveLength(1);
  m.capabilities!.referenceLimit = 16;
  expect(() => inputFor(p, task, m)).toThrow("16");
  delete m.capabilities!.referenceLimit;
  m.capabilities!.referenceSeconds = 10;
  expect(() => inputFor(p, task, m)).toThrow("10 秒");
  delete m.capabilities!.referenceSeconds;
  task.inputs = [
    {
      key: "v",
      assetId: "v",
      role: "video-reference",
      purpose: "motion",
      start: 0,
      end: 0.5,
    },
  ];
  expect(() => inputFor(p, task, m)).not.toThrow();
});
