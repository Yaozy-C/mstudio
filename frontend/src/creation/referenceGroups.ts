import { referenceAsset } from "../previs/media";
import type { Project, Reference } from "../model";

// Short video references travel as one concatenated file, in selection order.
// Keep original indices so image roles and source purposes retain their mapping.
export function referenceGroups(
  project: Project,
  refs: Reference[],
): number[][] {
  const videos = refs.flatMap((ref, index) => {
    const asset = referenceAsset(project, ref.assetId);
    return asset?.kind === "video"
      ? [{ index, seconds: (ref.end ?? asset.duration) - (ref.start ?? 0) }]
      : [];
  });
  const merge = videos.length > 1 && videos.some((v) => v.seconds < 2);
  const indices = new Set(merge ? videos.map((v) => v.index) : []);
  return refs.flatMap((_, index) =>
    !indices.has(index)
      ? [[index]]
      : index === videos[0].index
        ? [videos.map((v) => v.index)]
        : [],
  );
}
