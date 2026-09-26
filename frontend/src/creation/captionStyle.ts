import type { Caption } from "../model";
export const captionFonts = {
  sans: {
    name: "黑体",
    family: '"PingFang SC", "Microsoft YaHei", sans-serif',
  },
  serif: { name: "宋体", family: '"Songti SC", SimSun, serif' },
  mono: { name: "等宽", family: 'Menlo, Consolas, "PingFang SC", monospace' },
};
export function captionKey(c: Caption) {
  return JSON.stringify([
    c.text,
    c.font ?? "sans",
    c.color ?? "#ffffff",
    c.fontSize ?? 0.048,
    c.x ?? 0.5,
    c.y ?? null,
    c.background ?? true,
  ]);
}
