import { t, useLanguage } from "../i18n";
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
  useLanguage();
  return (
    <span
      className={`missing-asset ${compact ? "compact" : ""}`}
      role="img"
      aria-label={t("文件缺失：{v0}", { v0: asset.name })}
      title={t("文件缺失：{v0}\n将文件放回原位置后会自动恢复。", {
        v0: asset.path,
      })}
    >
      <FileX size={compact ? 24 : 36} />
      <strong>{t("文件缺失")}</strong>
      {!compact && (
        <>
          <span>{asset.name}</span>
          <small>
            {t("文件已删除或存储目录不可用。放回原位置后会自动恢复。")}
          </small>
        </>
      )}
    </span>
  );
}
