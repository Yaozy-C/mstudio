import { type Asset, type BoardNode, type Project } from "../model";
import type { ShotTake } from "./types";
export const shotsOf = (p: Project, screenplayId: string) =>
  p.nodes
    .filter((n) => n.kind === "shot" && n.shot?.screenplayId === screenplayId)
    .sort((a, b) => a.shot!.order - b.shot!.order);
export const shotBasis = (n: BoardNode) =>
  JSON.stringify([
    n.text,
    n.shot?.dialogue,
    n.shot?.duration,
    n.references?.map((r) => [r.assetId, r.purpose, r.start, r.end]),
    ...(n.shot?.prompt ? [n.shot.prompt] : []),
  ]);
export function takesOf(n: BoardNode): ShotTake[] {
  return n.shot?.takes ?? [];
}
export function addShotResult(
  p: Project,
  asset: Asset,
  shotId?: string,
  source?: BoardNode,
): Project {
  const assets = p.assets.some((a) => a.id === asset.id)
    ? p.assets
    : [...p.assets, asset];
  const shot = p.nodes.find((n) => n.id === shotId && n.kind === "shot");
  if (!shot?.shot) return { ...p, assets };
  const takes = takesOf(shot);
  const take = {
    assetId: asset.id,
    trimIn: 0,
    trimOut: asset.kind === "image" ? shot.shot.duration : asset.duration,
    basis: source ? shotBasis(source) : undefined,
  };
  return {
    ...p,
    assets,
    nodes: p.nodes.map((n) =>
      n.id === shot.id
        ? {
            ...n,
            shot: {
              ...shot.shot!,
              takes: [...takes.filter((t) => t.assetId !== asset.id), take],
            },
          }
        : n,
    ),
  };
}
