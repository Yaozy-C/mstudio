import { expect, test } from "bun:test";
import {
  animationPlan,
  animationState,
  captionWords,
  validAnimation,
} from "./captionAnimation";
import type { Caption } from "../model";
const base: Caption = { id: "a", text: "你好 world 👩‍💻", start: 4, end: 7 };
test("entrances finish at identity and loops match at every supported frame rate", () => {
  for (const animation of ["pop", "rise"] as const) {
    const c = { ...base, animation };
    expect(animationState(c, 0).opacity).toBe(0);
    expect(animationState(c, 1)).toMatchObject({ scale: 1, opacity: 1, dy: 0 });
  }
  for (const fps of [24, 25, 30, 60])
    for (const animation of ["shake", "glow"] as const) {
      const c = { ...base, animation };
      const plan = animationPlan(c, fps);
      expect(plan.loop).toBe(true);
      expect(plan.frames.reduce((sum, f) => sum + f.duration, 0)).toBeCloseTo(
        1,
      );
      expect(animationState(c, 0.25)).toEqual(animationState(c, 1.25));
    }
});
test("typewriter preserves complete graphemes and event plan equals sampled preview", () => {
  const c: Caption = { ...base, animation: "typewriter" };
  for (const fps of [24, 25, 30, 60]) {
    const plan = animationPlan(c, fps);
    for (let f = 0; f < 3 * fps; f++) {
      const time = f / fps;
      const segment = [...plan.frames]
        .reverse()
        .find((v) => v.time <= time + 1e-8)!;
      expect(animationState(c, segment.time).visible).toBe(
        animationState(c, time).visible,
      );
    }
  }
  const shown = c.text.slice(0, animationState(c, 2.8).visible);
  expect(shown).toBe(c.text);
  expect(animationPlan({ ...c, end: 86400 }, 60).frames.length).toBeLessThan(
    20,
  );
});
test("highlight honors supplied relative timing including silent gaps, otherwise labels use evenly spaced tokens", () => {
  const c: Caption = {
    ...base,
    text: "你好世界",
    animation: "highlight",
    words: [
      { text: "你好", start: 0.2, end: 0.8 },
      { text: "世界", start: 1.4, end: 2.9 },
    ],
  };
  expect(validAnimation(c)).toBe(true);
  expect(animationState(c, 0.5)).toMatchObject({ from: 0, to: 2 });
  expect(animationState(c, 1)).toMatchObject({ from: -1, to: -1 });
  expect(animationState(c, 1.5)).toMatchObject({ from: 2, to: 4 });
  expect(
    validAnimation({ ...c, words: [{ text: c.text, start: 0, end: 9 }] }),
  ).toBe(false);
  expect(
    captionWords(base)
      .map((w) => w.text)
      .join(""),
  ).toBe(base.text);
});
