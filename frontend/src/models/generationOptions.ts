import definitions from "./config/generation-options.json";
import type { MediaModel } from "./mediaRegistry";
import { pointerGet } from "./capabilities";

export type GenerationValue = string | number | boolean;
export type GenerationOption = {
  key: string;
  label: string;
  path: string;
  type: "string" | "number" | "integer" | "boolean";
  values?: GenerationValue[];
  min?: number;
  max?: number;
};
export function generationOptions(model: MediaModel): GenerationOption[] {
  return (
    (
      definitions as {
        plugin: string;
        model?: string;
        endpoints?: string[];
        options: GenerationOption[];
      }[]
    ).find(
      (d) =>
        d.plugin === model.plugin &&
        (!d.model || d.model === model.params.model) &&
        (!d.endpoints || d.endpoints.includes(model.endpoint)),
    )?.options ?? []
  );
}

export function generationOptionDefaults(model: MediaModel) {
  return Object.fromEntries(
    generationOptions(model).flatMap((option) => {
      const value = pointerGet(model.params, option.path);
      return value === undefined
        ? []
        : [
            [
              option.key,
              option.type === "boolean" && typeof value === "string"
                ? value === "true"
                : ["integer", "number"].includes(option.type)
                  ? Number(value)
                  : value,
            ],
          ];
    }),
  );
}

export function updateGenerationOption(
  selected: Record<string, GenerationValue>,
  key: string,
  value?: GenerationValue,
) {
  const options = { ...selected };
  if (value === undefined) delete options[key];
  else options[key] = value;
  return options;
}

export function validateGenerationOption(
  option: GenerationOption,
  value: unknown,
) {
  const validType =
    option.type === "integer"
      ? Number.isInteger(value)
      : typeof value === option.type;
  if (
    !validType ||
    (typeof value === "number" &&
      (!Number.isFinite(value) ||
        (option.min !== undefined && value < option.min) ||
        (option.max !== undefined && value > option.max))) ||
    (option.values && !option.values.includes(value as GenerationValue))
  )
    throw new Error(`生成参数「${option.label}」的取值不受支持`);
}
