import type { Caption } from "../model";
export const captionFonts = {
  sans: {
    name: "黑体",
    family: '"PingFang SC", "Microsoft YaHei", sans-serif',
  },
  serif: { name: "宋体", family: '"Songti SC", SimSun, serif' },
  hand: { name: "手写楷体", family: '"Kaiti SC", STKaiti, KaiTi, serif' },
  mono: { name: "等宽", family: 'Menlo, Consolas, "PingFang SC", monospace' },
};
export const defaultCaptionStyle = {
  font: "sans",
  color: "#ffffff",
  fontSize: 0.048,
  fontWeight: 600,
  background: true,
  backgroundColor: "#000000",
  backgroundOpacity: 0.65,
  backgroundRadius: 0,
  strokeColor: "#151515",
  strokeWidth: 0,
  shadowColor: "#000000",
  shadowBlur: 0,
  shadowOffset: 0,
} satisfies Partial<Caption>;
export type CaptionAppearance = Pick<Caption, keyof typeof defaultCaptionStyle>;
export function captionAppearance(c: Caption): CaptionAppearance {
  return Object.fromEntries(
    Object.entries(defaultCaptionStyle).map(([key, value]) => [
      key,
      c[key as keyof Caption] ??
        (key === "shadowBlur" && c.background === false ? 0.08 : value),
    ]),
  ) as CaptionAppearance;
}
export const captionPresets: {
  id: string;
  name: string;
  group: string;
  style: CaptionAppearance;
}[] = [
  {
    id: "clean",
    name: "清晰口播",
    group: "口播",
    style: {
      ...defaultCaptionStyle,
      background: false,
      strokeWidth: 0.06,
      shadowBlur: 0.08,
    },
  },
  {
    id: "yellow",
    name: "醒目黄字",
    group: "口播",
    style: {
      ...defaultCaptionStyle,
      background: false,
      color: "#ffe14a",
      fontWeight: 900,
      strokeWidth: 0.075,
    },
  },
  {
    id: "pill",
    name: "圆角底板",
    group: "口播",
    style: {
      ...defaultCaptionStyle,
      backgroundRadius: 0.35,
      backgroundOpacity: 0.8,
    },
  },
  {
    id: "comic",
    name: "元气漫画",
    group: "花字",
    style: {
      ...defaultCaptionStyle,
      background: false,
      color: "#fff294",
      fontWeight: 900,
      strokeWidth: 0.09,
      shadowColor: "#ff654a",
      shadowOffset: 0.1,
    },
  },
  {
    id: "pink",
    name: "桃桃气泡",
    group: "花字",
    style: {
      ...defaultCaptionStyle,
      color: "#a92d65",
      backgroundColor: "#ffe1ed",
      backgroundOpacity: 1,
      backgroundRadius: 0.55,
      fontWeight: 800,
    },
  },
  {
    id: "mint",
    name: "薄荷便签",
    group: "花字",
    style: {
      ...defaultCaptionStyle,
      color: "#153c35",
      backgroundColor: "#bbf7dc",
      backgroundOpacity: 1,
      backgroundRadius: 0.12,
    },
  },
  {
    id: "sale",
    name: "促销红牌",
    group: "花字",
    style: {
      ...defaultCaptionStyle,
      backgroundColor: "#dc303e",
      backgroundOpacity: 1,
      backgroundRadius: 0.14,
      fontWeight: 900,
    },
  },
  {
    id: "neon",
    name: "蓝调霓虹",
    group: "花字",
    style: {
      ...defaultCaptionStyle,
      background: false,
      color: "#dbfcff",
      strokeColor: "#28cbe8",
      strokeWidth: 0.035,
      shadowColor: "#23d8ff",
      shadowBlur: 0.32,
    },
  },
  {
    id: "cinema",
    name: "电影旁白",
    group: "叙事",
    style: {
      ...defaultCaptionStyle,
      font: "serif",
      background: false,
      fontWeight: 600,
      color: "#f5eedf",
      shadowBlur: 0.1,
      shadowOffset: 0.025,
    },
  },
  {
    id: "ink",
    name: "水墨题记",
    group: "叙事",
    style: {
      ...defaultCaptionStyle,
      font: "hand",
      color: "#29241e",
      backgroundColor: "#f3ebd9",
      backgroundOpacity: 0.94,
      backgroundRadius: 0.08,
    },
  },
  {
    id: "diary",
    name: "手写日记",
    group: "叙事",
    style: {
      ...defaultCaptionStyle,
      font: "hand",
      background: false,
      color: "#fff3d9",
      shadowBlur: 0.1,
      strokeWidth: 0.025,
    },
  },
  {
    id: "terminal",
    name: "复古终端",
    group: "叙事",
    style: {
      ...defaultCaptionStyle,
      font: "mono",
      color: "#c0ff95",
      backgroundColor: "#15221a",
      backgroundOpacity: 0.9,
      backgroundRadius: 0.08,
    },
  },
];
export function captionKey(c: Caption) {
  return JSON.stringify([
    c.text,
    captionAppearance(c),
    c.x ?? 0.5,
    c.y ?? null,
  ]);
}
