export const gradeControls = {
  exposure: ["曝光", -3, 3],
  temperature: ["色温", -100, 100],
  tint: ["色调", -100, 100],
  contrast: ["明暗对比", -100, 100],
  saturation: ["饱和度", -100, 100],
  vibrance: ["自然饱和度", -100, 100],
  shadows: ["阴影", -100, 100],
  highlights: ["高光", -100, 100],
  whites: ["白色色阶", -100, 100],
  blacks: ["黑色色阶", -100, 100],
  balance: ["分区平衡", -100, 100],
} as const;
export type Grade = Record<keyof typeof gradeControls, number> & {
  hsl: number[][];
  curves: number[][];
  wheels: number[][];
};
export function defaultGrade(): Grade {
  return {
    ...Object.fromEntries(Object.keys(gradeControls).map((k) => [k, 0])),
    hsl: Array.from({ length: 8 }, () => [0, 0, 0]),
    curves: Array.from({ length: 4 }, () => [0.25, 0.5, 0.75]),
    wheels: Array.from({ length: 3 }, () => [0, 0, 0]),
  } as Grade;
}
export function patchGrade(
  current: Grade | undefined,
  raw: unknown,
): Grade | undefined {
  if (raw === null) return undefined;
  if (!raw || typeof raw !== "object" || Array.isArray(raw))
    throw new Error("调色方案必须是对象");
  const result = structuredClone(current ?? defaultGrade());
  const valid = (v: unknown, min: number, max: number): v is number =>
    typeof v === "number" && Number.isFinite(v) && v >= min && v <= max;
  for (const [key, value] of Object.entries(raw)) {
    if (Object.hasOwn(gradeControls, key)) {
      const k = key as keyof typeof gradeControls;
      const [, min, max] = gradeControls[k];
      if (!valid(value, min, max)) throw new Error(`${key} 超出允许范围`);
      result[k] = value;
    } else if (key === "hsl" || key === "curves" || key === "wheels") {
      const count = { hsl: 8, curves: 4, wheels: 3 }[key];
      if (
        !Array.isArray(value) ||
        value.length !== count ||
        value.some(
          (row) =>
            !Array.isArray(row) ||
            row.length !== 3 ||
            row.some((v, i) =>
              key === "curves"
                ? !valid(v, 0, 1)
                : key === "wheels"
                  ? !valid(v, i < 2 ? 0 : -100, i === 0 ? 360 : 100)
                  : !valid(v, -100, 100),
            ) ||
            (key === "curves" && (row[0] > row[1] || row[1] > row[2])),
        )
      )
        throw new Error(`${key} 控制点无效`);
      result[key] = structuredClone(value);
    } else throw new Error(`不支持的调色参数：${key}`);
  }
  return result;
}
