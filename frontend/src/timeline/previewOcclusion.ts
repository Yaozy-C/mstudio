export type Bounds = { x: number; y: number; width: number; height: number };
export function fittedVideoRect(
  stage: Bounds,
  width: number,
  height: number,
): Bounds {
  const scale = Math.min(stage.width / width, stage.height / height);
  const w = width * scale,
    h = height * scale;
  return {
    x: stage.x + (stage.width - w) / 2,
    y: stage.y + (stage.height - h) / 2,
    width: w,
    height: h,
  };
}
export function intersects(a: Bounds, b: Bounds) {
  return (
    a.width > 0 &&
    a.height > 0 &&
    b.width > 0 &&
    b.height > 0 &&
    a.x < b.x + b.width &&
    b.x < a.x + a.width &&
    a.y < b.y + b.height &&
    b.y < a.y + a.height
  );
}
export function previewCovered(video: Bounds) {
  return Array.from(
    document.querySelectorAll<HTMLElement>(
      '.modal-backdrop, [role="dialog"], [role="menu"], [data-radix-popper-content-wrapper]',
    ),
  ).some((el) => {
    const style = getComputedStyle(el);
    return (
      style.visibility !== "hidden" &&
      style.display !== "none" &&
      intersects(video, el.getBoundingClientRect())
    );
  });
}
