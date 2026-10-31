import { StudioSelect } from "../ui/StudioSelect";
import { t } from "../i18n";
import {
  generationOptions,
  generationOptionDefaults,
  updateGenerationOption,
  type GenerationValue,
} from "../models/generationOptions";
import type { MediaModel } from "../models/mediaRegistry";
import type { GenerationParameters } from "./parameters";

export function GenerationOptions({
  model,
  parameters,
  change,
}: {
  model: MediaModel;
  parameters: GenerationParameters;
  change: (options: Record<string, GenerationValue>) => void;
}) {
  const defaults = generationOptionDefaults(model);
  const selected = parameters.options ?? {};
  const update = (key: string, value?: GenerationValue) => {
    change(updateGenerationOption(selected, key, value));
  };
  const labelValue = (value: GenerationValue) =>
    value === true || value === "true"
      ? t("开启")
      : value === false || value === "false"
        ? t("关闭")
        : String(value);
  return generationOptions(model).map((option) => {
    const value = selected[option.key] ?? defaults[option.key];
    const choices =
      option.values ?? (option.type === "boolean" ? [true, false] : undefined);
    return (
      <label key={option.key} className="generation-setting-field">
        {t(option.label)}
        {choices ? (
          <StudioSelect
            label={t(option.label)}
            value={value === undefined ? "" : JSON.stringify(value)}
            options={[
              { value: "", label: t("默认") },
              ...choices.map((v) => ({
                value: JSON.stringify(v),
                label: labelValue(v),
              })),
            ]}
            onValueChange={(v) =>
              update(
                option.key,
                v ? (JSON.parse(v) as GenerationValue) : undefined,
              )
            }
          />
        ) : (
          <input
            aria-label={t(option.label)}
            type={option.type === "string" ? "text" : "number"}
            value={value === undefined ? "" : String(value)}
            min={option.min}
            max={option.max}
            step={option.type === "integer" ? 1 : "any"}
            placeholder={t("默认")}
            onChange={(e) =>
              update(
                option.key,
                e.target.value === ""
                  ? undefined
                  : option.type === "string"
                    ? e.target.value
                    : Number(e.target.value),
              )
            }
          />
        )}
      </label>
    );
  });
}
