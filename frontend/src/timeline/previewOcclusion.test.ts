import { expect, test } from "bun:test";
import { fittedVideoRect, intersects } from "./previewOcclusion";
test("sidebar popovers and timeline menus do not hide the video", () => {
  const video = fittedVideoRect(
    { x: 200, y: 100, width: 1500, height: 800 },
    1080,
    1920,
  );
  expect(video).toEqual({ x: 725, y: 100, width: 450, height: 800 });
  expect(intersects(video, { x: 1500, y: 600, width: 360, height: 480 })).toBe(
    false,
  );
  expect(intersects(video, { x: 250, y: 500, width: 300, height: 350 })).toBe(
    false,
  );
  expect(intersects(video, { x: 900, y: 300, width: 400, height: 200 })).toBe(
    true,
  );
  expect(intersects(video, { x: 900, y: 300, width: 0, height: 0 })).toBe(
    false,
  );
});
