import type { BoardNode } from "../model";
import { nodeSize } from "./geometry";
export function fitView(
  nodes: BoardNode[],
  size: { width: number; height: number },
) {
  if (!nodes.length) return { x: 32, y: 32, scale: 1 };
  let minX = Infinity,
    minY = Infinity,
    maxX = -Infinity,
    maxY = -Infinity;
  for (const n of nodes) {
    const fallback = nodeSize(n);
    const s = {
      width: n.width ?? fallback.width,
      height: n.height ?? fallback.height,
    };
    minX = Math.min(minX, n.x);
    minY = Math.min(minY, n.y);
    maxX = Math.max(maxX, n.x + s.width);
    maxY = Math.max(maxY, n.y + s.height);
  }
  const w = maxX - minX,
    h = maxY - minY;
  const scale = Math.max(
    0.05,
    Math.min(
      1,
      Math.max(1, size.width - 64) / w,
      Math.max(1, size.height - 100) / h,
    ),
  );
  return {
    x: (size.width - w * scale) / 2 - minX * scale,
    y: 32 + Math.max(0, size.height - 100 - h * scale) / 2 - minY * scale,
    scale,
  };
}
