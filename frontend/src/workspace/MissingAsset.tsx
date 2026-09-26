import { FileX } from "@phosphor-icons/react";
import type { Asset } from "../model";
import "../styles/missing-asset.css";

export function MissingAsset({
  asset,
  compact = false,
}: {
  asset: Asset;
  compact?: boolean;
}) {
  return (
    <span
      className={`missing-asset ${compact ? "compact" : ""}`}
      role="img"
      aria-label={`文件缺失：${asset.name}`}
      title={`文件缺失：${asset.path}\n将文件放回原位置后会自动恢复。`}
    >
      <FileX size={compact ? 24 : 36} />
      <strong>文件缺失</strong>
      {!compact && (
        <>
          <span>{asset.name}</span>
          <small>文件已删除或存储目录不可用。放回原位置后会自动恢复。</small>
        </>
      )}
    </span>
  );
}
