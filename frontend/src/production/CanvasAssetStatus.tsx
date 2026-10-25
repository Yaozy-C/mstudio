import { CheckCircle } from "@phosphor-icons/react";
import { t } from "../i18n";
import type { Asset } from "../model";

export function CanvasAssetStatus({
  asset,
  inTimeline,
}: {
  asset?: Asset;
  inTimeline: boolean;
}) {
  if (!asset) return null;
  return (
    <>
      {inTimeline && (
        <span
          className="card-timeline-usage"
          role="img"
          aria-label={t("已加入时间线")}
          title={t("已加入时间线")}
        >
          <CheckCircle size={20} weight="fill" aria-hidden="true" />
        </span>
      )}
    </>
  );
}
