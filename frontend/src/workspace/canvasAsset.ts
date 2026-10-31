import { uid, type Asset, type BoardNode, type Project } from "../model";
import { canvasViewport } from "../canvas/viewportMemory";
export function canvasAsset(asset: Asset, index: number): BoardNode {
  return {
    id: uid(),
    kind: "asset",
    assetId: asset.id,
    x: (index % 3) * 300,
    y: Math.floor(index / 3) * 260,
    title: asset.name,
    text: "",
  };
}
export function canvasNote(project: Project, text: string): BoardNode {
  const view = canvasViewport(project);
  return {
    id: uid(),
    kind: "note",
    x: (Math.max(330, window.innerWidth / 2 - 180) - view.x) / view.scale,
    y: (160 - view.y) / view.scale,
    title: "草稿笔记",
    text,
    width: 360,
    height: 300,
  };
}
