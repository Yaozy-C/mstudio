import type { Visual } from "../model";
/** Browser development preview; desktop/export use the shared native filter graph. */
export function visualCss(v?: Visual) {
  if (!v) return {};
  const effect =
    v.effect === "grayscale"
      ? "grayscale(1)"
      : v.effect === "sepia"
        ? "sepia(1)"
        : v.effect === "blur"
          ? "blur(3px)"
          : "";
  return {
    filter: `brightness(${1 + v.brightness}) contrast(${v.contrast}) saturate(${v.saturation}) ${effect}`,
    maskImage:
      v.effect === "vignette"
        ? "radial-gradient(ellipse, black 35%, #0008 100%)"
        : undefined,
  };
}
