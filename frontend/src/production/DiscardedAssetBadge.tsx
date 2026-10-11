import { Prohibit } from "@phosphor-icons/react";
import { t } from "../i18n";
import "./discarded-asset-badge.css";

export function DiscardedAssetBadge() {
  return (
    <span
      className="discarded-asset-badge"
      role="img"
      aria-label={t("已废弃，不能用于生成")}
      title={t("已废弃，不能用于生成")}
    >
      <Prohibit size={18} weight="regular" aria-hidden="true" />
    </span>
  );
}
