import { failure } from "../errors/failure";
import type { MediaModel } from "../models/mediaRegistry";
import {
  choices,
  described,
  integer,
  object,
  type Schema,
} from "../domain/schema";
import {
  copyParameters,
  defaultDurationRange,
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
  durationRange?: { min: number; max: number };
};

// Shared by model capability discovery and the model-facing operation schema.
export function parameterSchema(fields: ParameterVocabulary) {
  const range = fields.durationRange ?? defaultDurationRange;
  const properties: Record<string, Schema> = {};
  if (fields.ratios.length)
    properties.aspectRatio = described(
      choices(...fields.ratios),
      "Output width:height ratio, e.g. 9:16. Use the selected model's enum; first-frame video may derive its ratio from the image.",
    );
  if (fields.resolutions.length)
    properties.resolution = described(
      choices(...fields.resolutions),
      "Output resolution label, case-sensitive: use uppercase P or K exactly as listed (e.g. 1080P, not 1080p). Only values listed by the selected model are valid; 1K/2K/4K are labels, not pixel dimensions.",
    );
  if (fields.duration)
    properties.duration = described(
      integer(range.min, range.max),
      `Generated source clip length in whole seconds, ${range.min} through ${range.max} for the selected model. Editorial shot length is separate. Unless the user explicitly fixes the generated source length, choose the shortest supported duration covering the shot (5.3-second cut -> duration=6; 4-second cut -> duration=5), without another confirmation. Keep essential action inside the editorial interval, leave any surplus as a hold, and preserve shot/timeline timing. An explicitly fixed source length outside the supported range is a capability conflict; do not silently replace it.`,
    );
  if (fields.customSize) {
    properties.width = described(
      integer(16, 3840),
      "Image width in pixels, multiple of 16. Supply height together; only for models exposing custom size. Maximum ratio 3:1; total pixels 655360–8294400.",
    );
    properties.height = described(
      integer(16, 3840),
      "Image height in pixels, multiple of 16. Supply width together; only for models exposing custom size. Maximum ratio 3:1; total pixels 655360–8294400.",
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
  durationRange: { min: number; max: number };
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
    duration: !!controls.duration,
    durationRange: {
      min: controls.duration?.min ?? defaultDurationRange.min,
      max: controls.duration?.max ?? defaultDurationRange.max,
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
      ? "Specify width and height together, multiples of 16; at most 3840 per side; ratio at most 3:1; 655360–8294400 total pixels."
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
  // Codex manages output settings; inherited task overrides must not block it.
  if (model.plugin === "codex-image") return {};
  const controls = resolvedControls(model);
  const { aspectRatio, resolution, duration, width, height } = value;
  if (
    aspectRatio &&
    !(controls.aspectRatio?.values ?? []).includes(aspectRatio)
  )
    throw failure("VALIDATION_FAILED", "当前模型不支持这个比例，请重新设置");
  if (resolution && !(controls.resolution?.values ?? []).includes(resolution))
    throw failure("VALIDATION_FAILED", "当前模型不支持这个尺寸，请重新设置");
  const range = {
    min: controls.duration?.min ?? defaultDurationRange.min,
    max: controls.duration?.max ?? defaultDurationRange.max,
  };
  if (
    duration !== undefined &&
    (!controls.duration ||
      !Number.isInteger(duration) ||
      duration < range.min ||
      duration > range.max)
  )
    throw failure(
      "VALIDATION_FAILED",
      `当前模型的视频时长须为 ${range.min}–${range.max} 秒`,
    );
  if (width !== undefined || height !== undefined) {
    if (
      !controls.imageSize ||
      !width ||
      !height ||
      !Number.isInteger(width) ||
      !Number.isInteger(height) ||
      width % 16 ||
      height % 16 ||
      Math.max(width, height) > 3840 ||
      Math.max(width / height, height / width) > 3 ||
      width * height < 655360 ||
      width * height > 8294400
    )
      throw failure(
        "VALIDATION_FAILED",
        "尺寸须为 16 的倍数，最长边不超过 3840；比例不超过 3:1，总像素为 65.5–829 万",
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
