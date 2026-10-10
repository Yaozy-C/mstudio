import { failure } from "../errors/failure";
import type { MediaModel } from "../models/mediaRegistry";
import {
  choices,
  described,
  integer,
  number,
  object,
  type Schema,
} from "../domain/schema";
import {
  copyParameters,
  frameRatioModel,
  pointerGet,
  pointerSet,
  resolvedControls,
  type ResolvedControls,
} from "../models/capabilities";

/** The vocabulary the Agent-facing parameter schema and the settings panel read. */
export type ParameterVocabulary = {
  ratios: string[];
  resolutions: string[];
  customSize: boolean;
  duration: boolean;
  sizeRange?: { min?: number; max?: number };
  durationRange?: { min?: number; max?: number };
};

// Shared by model capability discovery and the model-facing operation schema.
export function parameterSchema(fields: ParameterVocabulary) {
  const range = fields.durationRange ?? {};
  const properties: Record<string, Schema> = {};
  if (fields.ratios.length)
    properties.aspectRatio = described(
      choices(...fields.ratios),
      "Output width:height ratio, e.g. 9:16. Use the selected model's enum; first-frame video may derive its ratio from the image.",
    );
  if (fields.resolutions.length)
    properties.resolution = described(
      choices(...fields.resolutions),
      "Output resolution label. Use the exact case-sensitive values in the selected model configuration.",
    );
  if (fields.duration)
    properties.duration = described(
      { ...number(range.min, range.max), exclusiveMinimum: 0 },
      "Generated source clip length in seconds. Use the configured model bounds when present. Editorial shot length is separate.",
    );
  if (fields.customSize) {
    properties.width = described(
      integer(fields.sizeRange?.min ?? 1, fields.sizeRange?.max),
      "Image width in pixels. Supply height together; use the selected model configuration.",
    );
    properties.height = described(
      integer(fields.sizeRange?.min ?? 1, fields.sizeRange?.max),
      "Image height in pixels. Supply width together; use the selected model configuration.",
    );
  }
  return described(
    object(properties),
    "Explicit generation overrides. Read mstudio_models for the chosen model's supported fields, values and configuredDefaults. Omitted fields inherit task/configured settings; project export settings and shot duration do not imply these overrides.",
  );
}

export type GenerationParameters = {
  aspectRatio?: string;
  resolution?: string;
  duration?: number;
  width?: number;
  height?: number;
};

export type ParameterFields = Omit<ParameterVocabulary, "durationRange"> & {
  durationRange: { min?: number; max?: number };
  frameRatio: boolean;
  supported: boolean;
  controls: ResolvedControls;
};

/**
 * Which controls this model exposes. The model declares them; the Gemini protocol supplies
 * its own fixed vocabulary because its request builder writes `generationConfig` itself.
 */
export function parameterFields(model?: MediaModel): ParameterFields {
  const controls = model
    ? resolvedControls(model)
    : ({
        aspectRatio: null,
        resolution: null,
        duration: null,
        imageSize: null,
      } as const);
  return {
    ratios: controls.aspectRatio?.values ?? [],
    resolutions: controls.resolution?.values ?? [],
    customSize: !!controls.imageSize,
    sizeRange: {
      min: controls.imageSize?.min ?? undefined,
      max: controls.imageSize?.max ?? undefined,
    },
    duration: !!controls.duration,
    durationRange: {
      min: controls.duration?.min ?? undefined,
      max: controls.duration?.max ?? undefined,
    },
    frameRatio: model ? frameRatioModel(model) : false,
    supported: !!(
      controls.aspectRatio ||
      controls.resolution ||
      controls.duration ||
      controls.imageSize
    ),
    controls,
  };
}

export function parameterContract(model: MediaModel) {
  const fields = parameterFields(model);
  const controls = fields.controls;
  const configured = (control: { path: string } | null) =>
    control ? (pointerGet(model.params, control.path) ?? null) : null;
  return {
    parameters: parameterSchema(fields),
    configuredDefaults: {
      aspectRatio: configured(controls.aspectRatio),
      resolution: configured(controls.resolution),
      duration: configured(controls.duration),
      imageSize: configured(controls.imageSize),
    },
    constraints: fields.customSize
      ? "Specify positive integer width and height together; use configured bounds when present."
      : undefined,
    omission:
      "Omitted fields inherit the referenced task, then configured defaults; remaining defaults are chosen by the provider.",
  };
}

/**
 * Resolves task overrides into the request body. Values are written at the paths the model
 * declared, so nested request shapes work without a per-vendor branch here.
 */
export function taskParameters(
  model: MediaModel,
  value: GenerationParameters = {},
): Record<string, unknown> {
  const controls = resolvedControls(model);
  const { aspectRatio, resolution, duration, width, height } = value;
  if (
    controls.aspectRatio &&
    aspectRatio &&
    !(controls.aspectRatio?.values ?? []).includes(aspectRatio)
  )
    throw failure("VALIDATION_FAILED", "当前模型不支持这个比例，请重新设置");
  if (
    controls.resolution &&
    resolution &&
    !(controls.resolution?.values ?? []).includes(resolution)
  )
    throw failure("VALIDATION_FAILED", "当前模型不支持这个尺寸，请重新设置");
  const range = {
    min: controls.duration?.min ?? undefined,
    max: controls.duration?.max ?? undefined,
  };
  if (
    duration !== undefined &&
    controls.duration &&
    (duration <= 0 ||
      !Number.isFinite(duration) ||
      (range.min !== undefined && duration < range.min) ||
      (range.max !== undefined && duration > range.max))
  )
    throw failure("VALIDATION_FAILED", "视频时长须为正数并符合配置中的范围");
  if (controls.imageSize && (width !== undefined || height !== undefined)) {
    const { min, max } = controls.imageSize;
    if (
      ![width, height].every(
        (n) =>
          n !== undefined &&
          Number.isInteger(n) &&
          n > 0 &&
          (min === null || n >= min) &&
          (max === null || n <= max),
      )
    )
      throw failure(
        "VALIDATION_FAILED",
        "宽高须同时填写正整数并符合配置中的范围",
      );
  }
  const resolved = copyParameters(model.params);
  if (aspectRatio && controls.aspectRatio)
    pointerSet(resolved, controls.aspectRatio.path, aspectRatio);
  if (resolution && controls.resolution)
    pointerSet(resolved, controls.resolution.path, resolution);
  if (duration !== undefined && controls.duration)
    pointerSet(resolved, controls.duration.path, duration);
  if (width && height && controls.imageSize)
    pointerSet(resolved, controls.imageSize.path, { width, height });
  return resolved;
}
