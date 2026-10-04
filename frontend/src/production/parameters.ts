import { failure } from "../errors/failure";
import type { MediaModel } from "../models/mediaRegistry";
import {
  choices,
  described,
  integer,
  object,
  type Schema,
} from "../domain/schema";

export const imageRatios = ["1:1", "9:16", "16:9", "3:4", "4:3", "2:3", "3:2"];
export const videoRatios = ["9:16", "16:9", "1:1", "3:4", "4:3", "21:9"];
export const imageResolutions = ["1K", "2K", "4K"];
export const videoResolutions = ["480P", "768P", "2K", "4K"];
export const videoMaxResolutions = ["480P", "768P", "1080P"];

export const videoDurationRange = { min: 5, max: 15 };
// Shared by model capability discovery and the model-facing operation schema.
export function parameterSchema(fields: ReturnType<typeof parameterFields>) {
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
      integer(videoDurationRange.min, videoDurationRange.max),
      "Generated source clip length in whole seconds, 5 through 15 for the currently supported video models. This is not the editorial shot length. A 4-second shot does not make duration=4 valid. Resolve a duration mismatch explicitly; do not silently change the shot or requested generation length.",
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
export function parameterFields(model?: MediaModel) {
  const gemini = model?.plugin === "gemini-native";
  const banana =
    model?.plugin === "fal" &&
    /^fal-ai\/nano-banana-2(?:\/edit)?$/.test(model.endpoint);
  const gpt =
    model?.plugin === "fal" &&
    /^openai\/gpt-image-2\.5\/(sunburst|flare)\/(edit|text-to-image)$/.test(
      model.endpoint,
    );
  const h3 =
    model?.plugin === "fal" &&
    /^minimax\/h3(?:-max)?\/(image-to-video|reference-to-video|text-to-video)$/.test(
      model.endpoint,
    );
  return {
    ratios:
      gemini || banana
        ? imageRatios
        : h3 && !model?.endpoint.endsWith("image-to-video")
          ? videoRatios
          : [],
    resolutions:
      gemini || banana
        ? imageResolutions
        : h3
          ? model?.endpoint.includes("h3-max")
            ? videoMaxResolutions
            : videoResolutions
          : [],
    customSize: !!gpt,
    duration: !!h3,
    frameRatio: !!h3 && !!model?.endpoint.endsWith("image-to-video"),
    supported: !!(gemini || banana || gpt || h3),
  };
}
export function parameterContract(model: MediaModel) {
  const fields = parameterFields(model);
  const params = model.params;
  const image = (
    params.generationConfig as
      { imageConfig?: Record<string, unknown> } | undefined
  )?.imageConfig;
  return {
    parameters: parameterSchema(fields),
    configuredDefaults: {
      aspectRatio: image?.aspectRatio ?? params.aspect_ratio,
      resolution: image?.imageSize ?? params.resolution,
      duration: params.duration,
      imageSize: params.image_size,
    },
    constraints: fields.customSize
      ? "Specify width and height together, multiples of 16; at most 3840 per side; ratio at most 3:1; 655360–8294400 total pixels."
      : undefined,
    omission:
      "Omitted fields inherit the referenced task, then configured defaults; remaining defaults are chosen by the provider.",
  };
}
export function taskParameters(
  model: MediaModel,
  value: GenerationParameters = {},
): Record<string, unknown> {
  const fields = parameterFields(model);
  const { aspectRatio, resolution, duration, width, height } = value;
  if (aspectRatio && !fields.ratios.includes(aspectRatio))
    throw failure("VALIDATION_FAILED", "当前模型不支持这个比例，请重新设置");
  if (resolution && !fields.resolutions.includes(resolution))
    throw failure("VALIDATION_FAILED", "当前模型不支持这个尺寸，请重新设置");
  if (
    duration !== undefined &&
    (!fields.duration ||
      !Number.isInteger(duration) ||
      duration < videoDurationRange.min ||
      duration > videoDurationRange.max)
  )
    throw failure("VALIDATION_FAILED", "当前模型的视频时长须为 5–15 秒");
  if (width !== undefined || height !== undefined) {
    if (
      !fields.customSize ||
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
  if (model.plugin === "gemini-native") {
    if (!aspectRatio && !resolution) return {};
    const config = model.params.generationConfig as
      Record<string, unknown> | undefined;
    return {
      generationConfig: {
        ...config,
        imageConfig: {
          ...((config?.imageConfig as object) ?? {}),
          ...(aspectRatio ? { aspectRatio } : {}),
          ...(resolution ? { imageSize: resolution } : {}),
        },
      },
    };
  }
  return {
    ...(aspectRatio ? { aspect_ratio: aspectRatio } : {}),
    ...(resolution ? { resolution } : {}),
    ...(duration !== undefined ? { duration } : {}),
    ...(width && height ? { image_size: { width, height } } : {}),
  };
}
