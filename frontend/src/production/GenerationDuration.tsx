import { t } from "../i18n";
export function GenerationDuration({
  value,
  range,
  onChange,
}: {
  value?: number;
  range: { min?: number; max?: number };
  onChange: (value?: number) => void;
}) {
  return (
    <label className="generation-setting-field">
      {t("视频时长")}
      <input
        type="number"
        min={range.min ?? 0}
        max={range.max}
        step="any"
        placeholder={t("沿用模型设置")}
        value={value ?? ""}
        onChange={(e) =>
          onChange(e.target.value ? Number(e.target.value) : undefined)
        }
      />
    </label>
  );
}
