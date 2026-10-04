import { array, choices, nullable, number, object } from "./schema";
const pair = (min: number, max: number) => array(number(min, max), 2, 2);
const grade = object({
  ...Object.fromEntries(
    [
      "temperature",
      "tint",
      "contrast",
      "saturation",
      "vibrance",
      "shadows",
      "highlights",
      "whites",
      "blacks",
      "balance",
    ].map((k) => [k, number(-100, 100)]),
  ),
  exposure: number(-3, 3),
  hsl: array(array(number(-100, 100), 3, 3), 8, 8),
  curves: array(array(number(0, 1), 3, 3), 4, 4),
  wheels: array(array(number(-100, 360), 3, 3), 3, 3),
});
export const visual = nullable(
  object({
    brightness: number(-0.5, 0.5),
    contrast: number(0.5, 1.5),
    saturation: number(0, 2),
    temperature: number(-1, 1),
    effect: choices("none", "grayscale", "sepia", "blur", "vignette"),
    grade: nullable(grade),
  }),
);
export const transitionKind = choices(
  "fade",
  "fadeblack",
  "fadewhite",
  "wipeleft",
  "wiperight",
  "slideleft",
  "slideright",
  "smoothleft",
  "smoothright",
  "circleopen",
  "circleclose",
  "dissolve",
  "custom",
  null,
);
export const design = object({
  mask: choices("uniform", "linear", "radial"),
  angle: number(-180, 180),
  center: pair(0, 1),
  feather: number(0.001, 1),
  curve: array(pair(0, 1), 8, 2),
  outgoingZoom: number(1, 4),
  incomingZoom: number(1, 4),
  outgoingOffset: pair(-1, 1),
  incomingOffset: pair(-1, 1),
});
