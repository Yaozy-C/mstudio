import { StudioSelect } from "../ui/StudioSelect";
import { t } from "../i18n";
export function GenerationDuration({
  value,
  range,
  choices = [],
  onChange,
}: {
  value?: number;
  range: { min?: number; max?: number };
  choices?: number[];
  onChange: (value?: number) => void;
}) {
  return (
    <label className="generation-setting-field">
      {t("视频时长")}
      {choices.length ? (
        <StudioSelect
          label={t("视频时长")}
          value={value === undefined ? "" : String(value)}
          options={[
            { value: "", label: t("默认") },
            ...(value !== undefined && !choices.includes(value)
              ? [
                  {
                    value: String(value),
                    label: `${value} ${t("秒")} · ${t("不支持")}`,
                    disabled: true,
                  },
                ]
              : []),
            ...choices.map((v) => ({
              value: String(v),
              label: `${v} ${t("秒")}`,
            })),
          ]}
          onValueChange={(v) => onChange(v ? Number(v) : undefined)}
        />
      ) : (
        <div className="generation-duration-input">
          <input
            aria-label={t("视频时长")}
            type="number"
            min={range.min ?? 0}
            max={range.max}
            step="any"
            placeholder={t("未指定")}
            value={value ?? ""}
            onChange={(e) =>
              onChange(e.target.value ? Number(e.target.value) : undefined)
            }
          />
          <span>{t("秒")}</span>
        </div>
      )}
    </label>
  );
}
