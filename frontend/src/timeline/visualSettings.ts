import type { Visual } from "../model";
export const defaultVisual: Visual = {
  brightness: 0,
  contrast: 1,
  saturation: 1,
  temperature: 0,
  effect: "none",
};
export function patchVisual(
  current: Visual | undefined,
  raw: unknown,
): Visual | undefined {
  if (raw === null) return undefined;
  if (!raw || typeof raw !== "object" || Array.isArray(raw))
    throw new Error("调色参数必须是对象");
  const fields = raw as Record<string, unknown>;
  const next = { ...defaultVisual, ...current };
  const ranges = {
    brightness: [-0.5, 0.5],
    contrast: [0.5, 1.5],
    saturation: [0, 2],
    temperature: [-1, 1],
  } as const;
  for (const [key, value] of Object.entries(fields)) {
    if (key === "effect") {
      if (
        typeof value !== "string" ||
        !["none", "grayscale", "sepia", "blur", "vignette"].includes(value)
      )
        throw new Error("不支持的画面特效");
      next.effect = value as Visual["effect"];
    } else {
      if (!(key in ranges)) throw new Error(`不支持的调色参数：${key}`);
      const k = key as keyof typeof ranges,
        [min, max] = ranges[k];
      if (
        typeof value !== "number" ||
        !Number.isFinite(value) ||
        value < min ||
        value > max
      )
        throw new Error(`调色参数 ${key} 超出范围`);
      next[k] = value;
    }
  }
  return next;
}
