export type TransitionDesign = {
  mask: "uniform" | "linear" | "radial";
  angle: number;
  center: [number, number];
  feather: number;
  curve: [number, number][];
  outgoingZoom: number;
  incomingZoom: number;
  outgoingOffset: [number, number];
  incomingOffset: [number, number];
};
export const defaultDesign = (): TransitionDesign => ({
  mask: "uniform",
  angle: 0,
  center: [0.5, 0.5],
  feather: 0.1,
  curve: [
    [0, 0],
    [1, 1],
  ],
  outgoingZoom: 1,
  incomingZoom: 1,
  outgoingOffset: [0, 0],
  incomingOffset: [0, 0],
});
export function readDesign(raw: unknown): TransitionDesign {
  if (!raw || typeof raw !== "object" || Array.isArray(raw))
    throw new Error("自定义转场必须提供 design");
  const value = { ...defaultDesign(), ...raw } as TransitionDesign;
  if (Object.keys(raw).some((k) => !Object.hasOwn(defaultDesign(), k)))
    throw new Error("未知转场设计参数");
  const range = (n: unknown, a: number, b: number) =>
    typeof n === "number" && Number.isFinite(n) && n >= a && n <= b;
  const pair = (v: unknown, a: number, b: number) =>
    Array.isArray(v) && v.length === 2 && v.every((n) => range(n, a, b));
  if (
    !["uniform", "linear", "radial"].includes(value.mask) ||
    !range(value.angle, -180, 180) ||
    !range(value.feather, 0.001, 1) ||
    !pair(value.center, 0, 1) ||
    !pair(value.outgoingOffset, -1, 1) ||
    !pair(value.incomingOffset, -1, 1) ||
    !range(value.outgoingZoom, 1, 4) ||
    !range(value.incomingZoom, 1, 4)
  )
    throw new Error("自定义转场数值超出范围");
  if (
    !Array.isArray(value.curve) ||
    value.curve.length < 2 ||
    value.curve.length > 8 ||
    value.curve.some((p) => !pair(p, 0, 1)) ||
    value.curve[0].some((n) => n !== 0) ||
    value.curve.at(-1)!.some((n) => n !== 1) ||
    value.curve.some(
      (p, i, a) => i > 0 && (p[0] <= a[i - 1][0] || p[1] < a[i - 1][1]),
    )
  )
    throw new Error("进度曲线须含 2–8 个递增点，从 [0,0] 到 [1,1]");
  return structuredClone(value);
}
