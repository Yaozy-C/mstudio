import { failure } from "../errors/failure";
import type { MediaModel } from "../models/mediaRegistry";

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
        ? ["1:1", "9:16", "16:9", "3:4", "4:3", "2:3", "3:2"]
        : h3 && !model?.endpoint.endsWith("image-to-video")
          ? ["9:16", "16:9", "1:1", "3:4", "4:3", "21:9"]
          : [],
    resolutions:
      gemini || banana
        ? ["1K", "2K", "4K"]
        : h3
          ? model?.endpoint.includes("h3-max")
            ? ["480P", "768P", "1080P"]
            : ["480P", "768P", "2K", "4K"]
          : [],
    customSize: !!gpt,
    duration: !!h3,
    frameRatio: !!h3 && !!model?.endpoint.endsWith("image-to-video"),
    supported: !!(gemini || banana || gpt || h3),
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
      duration < 5 ||
      duration > 15)
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
