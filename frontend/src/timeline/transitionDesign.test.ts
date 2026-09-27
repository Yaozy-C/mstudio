import { expect, test } from "bun:test";
import { defaultDesign, readDesign } from "./transitionDesign";
test("custom design retains independent motion and arbitrary monotonic progress", () => {
  const d = readDesign({
    mask: "radial",
    center: [0.3, 0.7],
    incomingZoom: 2,
    curve: [
      [0, 0],
      [0.3, 0.1],
      [0.8, 0.95],
      [1, 1],
    ],
  });
  expect(d.outgoingZoom).toBe(1);
  expect(d.incomingZoom).toBe(2);
  expect(d.curve[1]).toEqual([0.3, 0.1]);
  expect(defaultDesign().center).toEqual([0.5, 0.5]);
});
test("reject executable strings, invalid bounds and discontinuous endpoint contracts", () => {
  for (const raw of [
    undefined,
    { expr: "movie=foo" },
    { feather: 0 },
    { center: [NaN, 0.5] },
    { incomingZoom: 0 },
    {
      curve: [
        [0, 0],
        [0.5, 0.7],
        [0.4, 0.8],
        [1, 1],
      ],
    },
    {
      curve: [
        [0, 0],
        [0.5, 0.8],
        [0.8, 0.5],
        [1, 1],
      ],
    },
    {
      curve: [
        [0, 0],
        [1, 0.9],
      ],
    },
  ])
    expect(() => readDesign(raw)).toThrow();
});
