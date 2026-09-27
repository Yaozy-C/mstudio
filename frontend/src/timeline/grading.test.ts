import { expect, test } from "bun:test";
import { defaultGrade, patchGrade } from "./grading";
import { patchVisual, defaultVisual } from "./visualSettings";
test("grade edits merge only requested fields, retain effects and clear independently", () => {
  const original = {
    ...defaultVisual,
    effect: "vignette" as const,
    grade: { ...defaultGrade(), shadows: 20 },
  };
  const changed = patchVisual(original, { grade: { exposure: 0.5 } })!;
  expect(changed.grade?.shadows).toBe(20);
  expect(changed.grade?.exposure).toBe(0.5);
  expect(changed.effect).toBe("vignette");
  expect(original.grade.exposure).toBe(0);
  expect(patchVisual(changed, { grade: null })?.grade).toBeUndefined();
  expect(patchVisual(changed, { grade: null })?.effect).toBe("vignette");
});
test("reject malformed hue bands, nonmonotonic curves and invalid wheel ranges", () => {
  for (const raw of [
    { hsl: [[1, 2, 3]] },
    { exposure: 4 },
    { mystery: 1 },
    { tint: NaN },
    { curves: Array.from({ length: 4 }, () => [0.8, 0.5, 0.2]) },
    {
      wheels: [
        [0, -10, 0],
        [0, 0, 0],
        [0, 0, 0],
      ],
    },
  ])
    expect(() => patchGrade(undefined, raw)).toThrow();
  expect(patchGrade(undefined, { exposure: 1 })?.curves).toEqual(
    defaultGrade().curves,
  );
});
