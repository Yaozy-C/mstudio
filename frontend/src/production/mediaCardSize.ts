import type { Asset } from "../model";

export function mediaCardSize(asset: Asset) {
  if (
    ![asset.width, asset.height].every(
      (dimension) => Number.isFinite(dimension) && dimension > 0,
    )
  )
    throw new Error("素材画面尺寸无效");
  const width = asset.kind === "video" ? 260 : 250;
  // 12px padding + 1px border on both sides; 18px title + 10px gap.
  return { width, height: 54 + ((width - 26) * asset.height) / asset.width };
}
