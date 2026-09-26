import type { BoardNode } from "../model";
export function nodeSize(node: BoardNode) {
  return { width: 390, height: 230 };
  return {
    width: node.width || 260,
    height: node.height || (node.kind === "text" ? 280 : 218),
  };
}
