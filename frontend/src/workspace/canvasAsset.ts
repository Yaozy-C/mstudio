import { uid, type Asset, type BoardNode, type Project } from "../model";
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
export function canvasNote(
  project: Project,
  kind: "text" | "shot",
  text: string,
): BoardNode {
  return {
    id: uid(),
    kind,
    x:
      (Math.max(330, window.innerWidth / 2 - 180) - project.viewport.x) /
      project.viewport.scale,
    y: (160 - project.viewport.y) / project.viewport.scale,
    title:
      kind === "shot"
        ? `镜头 ${project.nodes.filter((n) => n.kind === "shot").length + 1}`
        : "创作脚本",
    text,
    width: 360,
    height: 300,
  };
}
