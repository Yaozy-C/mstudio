import { expect, test } from "bun:test";
import { codexImageSize } from "./codexImageSize";

test("common project frames use explicit 16-multiple sizes", () => {
  expect(codexImageSize(1920, 1080)).toBe("2048x1152");
  expect(codexImageSize(1080, 1920)).toBe("1152x2048");
  expect(codexImageSize(1024, 1024)).toBe("1024x1024");
  expect(codexImageSize(1080, 1350)).toBe("1024x1280");
  expect(codexImageSize(2560, 1080)).toBe("2016x864");
});

test("returned sizes satisfy the Codex image constraints", () => {
  for (const [width, height] of [
    [1920, 1080],
    [1080, 1920],
    [1000, 1000],
    [1600, 900],
    [2048, 858],
    [1200, 1600],
    [3840, 2160],
  ]) {
    const [w, h] = codexImageSize(width, height).split("x").map(Number);
    expect(w % 16).toBe(0);
    expect(h % 16).toBe(0);
    expect(w).toBeLessThanOrEqual(3840);
    expect(h).toBeLessThanOrEqual(3840);
    expect(w * h).toBeGreaterThanOrEqual(655_360);
    expect(w * h).toBeLessThanOrEqual(8_294_400);
    expect(w / h).toBeGreaterThanOrEqual(1 / 3);
    expect(w / h).toBeLessThanOrEqual(3);
  }
});
