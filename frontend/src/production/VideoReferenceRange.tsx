import { t } from "../i18n";
import type { ProductionInput } from "./types";

export function VideoReferenceRange({
  input,
  duration,
  change,
}: {
  input: ProductionInput;
  duration: number;
  change: (patch: Partial<ProductionInput>) => void;
}) {
  return (
    <div className="reference-video-range">
      <p>
        {t("原视频 {v0} 秒；请明确选择参考区间，不会自动裁短。", {
          v0: duration.toFixed(2),
        })}
      </p>
      <label>
        {t("起点（秒）")}
        <input
          type="number"
          min={0}
          max={duration}
          step="0.001"
          value={input.start ?? 0}
          onChange={(e) => change({ start: e.target.valueAsNumber })}
        />
      </label>
      <label>
        {t("终点（秒）")}
        <input
          type="number"
          min={0}
          max={duration}
          step="0.001"
          value={input.end ?? duration}
          onChange={(e) => change({ end: e.target.valueAsNumber })}
        />
      </label>
      <button type="button" onClick={() => change({ start: 0, end: duration })}>
        {t("使用完整视频")}
      </button>
    </div>
  );
}
