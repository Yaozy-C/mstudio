import { test, expect } from "bun:test";
import { fitPanel, resizePanel, compactPanel } from "./panelGeometry";
const viewport = { width: 1560, height: 980 };
test("agent can grow to the bottom margin instead of reserving timeline height", () => {
  const start = { x: 1192, y: 82, width: 356, height: 480 };
  const result = resizePanel(start, 0, 800, "height", viewport);
  expect(result.height).toBe(886);
  expect(result.y + result.height).toBe(968);
  expect(result.width).toBe(start.width);
});
test("resizing starts from visible dimensions after a smaller window clamps stored bounds", () => {
  const stored = { x: 1500, y: 130, width: 500, height: 1100 };
  const smallWindow = { width: 1100, height: 740 };
  const visible = fitPanel(stored, smallWindow);
  const resized = resizePanel(visible, 0, -40, "height", smallWindow);
  expect(resized.height).toBe(visible.height - 40);
  expect(visible.x + visible.width).toBeLessThanOrEqual(1088);
  expect(visible.y + visible.height).toBeLessThanOrEqual(728);
});
test("collapse remains anchored to the right and never destroys expanded dimensions", () => {
  const expanded = { x: 1192, y: 82, width: 356, height: 850 };
  const compact = compactPanel(expanded);
  expect(compact.width).toBe(132);
  expect(compact.height).toBe(44);
  expect(compact.x + compact.width).toBe(expanded.x + expanded.width);
  expect(fitPanel(expanded, viewport)).toEqual(expanded);
});
test("width and corner resizing keep the whole panel inside the viewport", () => {
  const start = { x: 500, y: 200, width: 400, height: 400 };
  const grown = resizePanel(start, 5000, 5000, "both", viewport);
  expect(grown.x + grown.width).toBe(1548);
  expect(grown.y + grown.height).toBe(968);
  const narrowed = resizePanel(start, -5000, 800, "width", viewport);
  expect(narrowed.width).toBe(260);
  expect(narrowed.height).toBe(start.height);
});
