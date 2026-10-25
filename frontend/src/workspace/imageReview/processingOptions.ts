export type ColorCorrection = "wavelet" | "lab" | "none";
export type ProcessingOptions = {
  vaeTile: 256 | 384 | 512;
  denoise: number;
  quantize8: boolean;
  colorCorrection: ColorCorrection;
};
const key = "mstudio-image-processing-options";
export function readProcessingOptions(): ProcessingOptions {
  try {
    const value = JSON.parse(localStorage.getItem(key) || "{}");
    return {
      vaeTile: [256, 384, 512].includes(value?.vaeTile) ? value.vaeTile : 512,
      denoise:
        typeof value?.denoise === "number" &&
        Number.isFinite(value.denoise) &&
        value.denoise >= 0 &&
        value.denoise <= 1
          ? value.denoise
          : 1,
      quantize8: value?.quantize8 === true,
      colorCorrection: ["wavelet", "lab", "none"].includes(
        value?.colorCorrection,
      )
        ? value.colorCorrection
        : "wavelet",
    };
  } catch {
    return {
      vaeTile: 512,
      denoise: 1,
      quantize8: false,
      colorCorrection: "wavelet",
    };
  }
}
export function saveProcessingOptions(value: ProcessingOptions) {
  try {
    localStorage.setItem(key, JSON.stringify(value));
  } catch {
    /* Preferences are optional. */
  }
}
