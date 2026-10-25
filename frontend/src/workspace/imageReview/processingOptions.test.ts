import { expect, test } from "bun:test";
import {
  readProcessingOptions,
  saveProcessingOptions,
} from "./processingOptions";

test("old preferences gain Wavelet; color choices persist and corrupt preferences recover", () => {
  const original = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
  let saved = '{"vaeTile":384,"quantize8":true}';
  try {
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: {
        getItem: () => saved,
        setItem: (_key: string, value: string) => {
          saved = value;
        },
      },
    });
    expect(readProcessingOptions()).toEqual({
      vaeTile: 384,
      denoise: 1,
      quantize8: true,
      colorCorrection: "wavelet",
    });
    for (const colorCorrection of ["wavelet", "lab", "none"] as const) {
      saveProcessingOptions({
        vaeTile: 512,
        denoise: 0.4,
        quantize8: false,
        colorCorrection,
      });
      expect(readProcessingOptions().colorCorrection).toBe(colorCorrection);
      expect(readProcessingOptions().denoise).toBe(0.4);
    }
    saved = '{"colorCorrection":"broken"}';
    expect(readProcessingOptions().colorCorrection).toBe("wavelet");
    for (const denoise of [0, 0.2, 0.4, 0.6, 1]) {
      saved = JSON.stringify({ denoise });
      expect(readProcessingOptions().denoise).toBe(denoise);
    }
    for (const denoise of [-1, 2, "0.4", null]) {
      saved = JSON.stringify({ denoise });
      expect(readProcessingOptions().denoise).toBe(1);
    }
    saved = "invalid JSON";
    expect(readProcessingOptions()).toEqual({
      vaeTile: 512,
      denoise: 1,
      quantize8: false,
      colorCorrection: "wavelet",
    });
  } finally {
    if (original) Object.defineProperty(globalThis, "localStorage", original);
    else Reflect.deleteProperty(globalThis, "localStorage");
  }
});
