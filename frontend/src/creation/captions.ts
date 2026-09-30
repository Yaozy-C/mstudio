import { animationState, validAnimation } from "./captionAnimation";
import { uid, type Caption } from "../model";
import { captionFonts, captionKey, captionAppearance } from "./captionStyle";
const images = new Map<string, string>();
/** Identical raster used by preview and native export; no platform subtitle-font mismatch. */
export function captionImage(
  input: string | Caption,
  width: number,
  height: number,
  time?: number,
) {
  const caption: Caption =
    typeof input === "string"
      ? { id: "", start: 0, end: 1, text: input }
      : input;
  const text = caption.text;
  const style = captionAppearance(caption);
  const state = time === undefined ? undefined : animationState(caption, time);
  const key = `${width}x${height}:${captionKey(caption)}:${JSON.stringify(state)}:${caption.highlightColor ?? "#ffe14a"}`;
  const cached = images.get(key);
  if (cached) return cached;
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d")!;
  let size = Math.round(Math.min(width, height) * (caption.fontSize ?? 0.048)),
    lines: string[] = [];
  let starts: number[] = [];
  const wrap = () => {
    ctx.font = `${style.fontWeight} ${size}px ${captionFonts[caption.font ?? "sans"].family}`;
    lines = [];
    starts = [0];
    let offset = 0;
    let line = "";
    for (const ch of text) {
      if (
        ch === "\n" ||
        (line && ctx.measureText(line + ch).width > width * 0.86)
      ) {
        lines.push(line);
        starts.push(offset + (ch === "\n" ? 1 : 0));
        line = ch === "\n" ? "" : ch;
      } else line += ch;
      offset += ch.length;
    }
    if (line) lines.push(line);
  };
  wrap();
  while (lines.length * size * 1.35 > height * 0.4 && size > 10) {
    size -= 2;
    wrap();
  }
  const lineHeight = size * 1.35,
    blockHeight = lines.length * lineHeight,
    y = Math.max(
      size * 0.2,
      Math.min(
        height - blockHeight - size * 0.2,
        caption.y === undefined
          ? height * 0.89 - blockHeight
          : height * caption.y - blockHeight / 2,
      ),
    );
  const x = Math.max(
    width * 0.07,
    Math.min(width * 0.93, width * (caption.x ?? 0.5)),
  );
  ctx.textAlign = "center";
  ctx.textBaseline = "top";
  const boxWidth = Math.min(
    width * 0.92,
    Math.max(0, ...lines.map((l) => ctx.measureText(l).width)) + size,
  );
  if (state) {
    ctx.translate(x + size * state.dx, y + blockHeight / 2 + size * state.dy);
    ctx.scale(state.scale, state.scale);
    ctx.translate(-x, -y - blockHeight / 2);
    ctx.globalAlpha = state.opacity;
  }
  if (style.background) {
    ctx.fillStyle = style.backgroundColor!;
    ctx.globalAlpha = style.backgroundOpacity! * (state?.opacity ?? 1);
    const box = [
      x - boxWidth / 2,
      y - size * 0.2,
      boxWidth,
      blockHeight + size * 0.4,
    ] as const;
    if (style.backgroundRadius) {
      ctx.beginPath();
      ctx.roundRect(...box, size * style.backgroundRadius);
      ctx.fill();
    } else ctx.fillRect(...box);
    ctx.globalAlpha = state?.opacity ?? 1;
  }
  ctx.fillStyle = style.color!;
  ctx.strokeStyle = style.strokeColor!;
  ctx.lineWidth = size * style.strokeWidth! * 2;
  ctx.lineJoin = "round";
  lines.forEach((l, i) => {
    ctx.shadowColor = state?.glow
      ? (caption.highlightColor ?? "#5fe5ff")
      : style.shadowColor!;
    ctx.shadowBlur =
      size *
      (state?.glow
        ? Math.max(state.glow, style.shadowBlur!)
        : style.shadowBlur!);
    ctx.shadowOffsetX = size * style.shadowOffset!;
    ctx.shadowOffsetY = size * style.shadowOffset!;
    const visible = state
      ? l.slice(0, Math.max(0, state.visible - starts[i]))
      : l;
    const left = x - ctx.measureText(l).width / 2;
    ctx.textAlign = state ? "left" : "center";
    if (style.strokeWidth) {
      ctx.strokeText(visible, state ? left : x, y + i * lineHeight);
      ctx.shadowBlur = 0;
      ctx.shadowOffsetX = ctx.shadowOffsetY = 0;
    }
    ctx.fillStyle = style.color!;
    ctx.fillText(visible, state ? left : x, y + i * lineHeight);
    if (state && state.from >= 0) {
      const from = Math.max(0, state.from - starts[i]),
        to = Math.min(l.length, state.to - starts[i]);
      if (to > from) {
        ctx.fillStyle = caption.highlightColor ?? "#ffe14a";
        ctx.fillText(
          l.slice(from, to),
          left + ctx.measureText(l.slice(0, from)).width,
          y + i * lineHeight,
        );
      }
    }
  });
  const url = canvas.toDataURL("image/png");
  if (images.size >= 32) images.delete(images.keys().next().value!);
  images.set(key, url);
  return url;
}
export function validCaption(c: Caption) {
  return (
    validAnimation(c) &&
    Number.isFinite(c.start) &&
    Number.isFinite(c.end) &&
    c.start >= 0 &&
    c.end > c.start &&
    c.end <= 86400 &&
    c.text.trim().length > 0 &&
    c.text.length <= 1000 &&
    (c.font === undefined || c.font in captionFonts) &&
    [c.color, c.strokeColor, c.shadowColor, c.backgroundColor].every(
      (v) => v === undefined || /^#[0-9a-f]{6}$/i.test(v),
    ) &&
    [
      c.strokeWidth,
      c.shadowBlur,
      c.shadowOffset,
      c.backgroundOpacity,
      c.backgroundRadius,
    ].every(
      (v) => v === undefined || (Number.isFinite(v) && v >= 0 && v <= 1),
    ) &&
    (c.fontWeight === undefined ||
      [400, 600, 800, 900].includes(c.fontWeight)) &&
    [c.x, c.y].every(
      (v) => v === undefined || (Number.isFinite(v) && v >= 0 && v <= 1),
    ) &&
    (c.fontSize === undefined ||
      (Number.isFinite(c.fontSize) && c.fontSize >= 0.02 && c.fontSize <= 0.12))
  );
}
const parseTime = (s: string) => {
  const m = s.match(/^(\d{1,2}):(\d{2}):(\d{2})[,.](\d{3})$/);
  if (!m || +m[2] > 59 || +m[3] > 59) throw new Error("字幕时间格式无效");
  return +m[1] * 3600 + +m[2] * 60 + +m[3] + +m[4] / 1000;
};
export function parseSrt(text: string): Caption[] {
  const blocks = text
    .replace(/^\uFEFF/, "")
    .replace(/\r/g, "")
    .trim()
    .split(/\n\s*\n/);
  if (blocks.length > 5000) throw new Error("字幕超过 5000 条");
  return blocks
    .map((block) => {
      const lines = block.split("\n");
      if (/^\d+$/.test(lines[0])) lines.shift();
      const times = (lines.shift() ?? "").split(/\s+-->\s+/);
      if (times.length !== 2) throw new Error("需要标准 SRT 字幕文件");
      const c = {
        id: uid(),
        start: parseTime(times[0]),
        end: parseTime(times[1]),
        text: lines.join("\n"),
      };
      if (!validCaption(c)) throw new Error("字幕为空、过长或时间无效");
      return c;
    })
    .sort((a, b) => a.start - b.start);
}
function time(s: number) {
  const ms = Math.round(s * 1000);
  return `${Math.floor(ms / 3600000)
    .toString()
    .padStart(
      2,
      "0",
    )}:${(Math.floor(ms / 60000) % 60).toString().padStart(2, "0")}:${(Math.floor(ms / 1000) % 60).toString().padStart(2, "0")},${(ms % 1000).toString().padStart(3, "0")}`;
}
export const toSrt = (captions: Caption[]) =>
  [...captions]
    .sort((a, b) => a.start - b.start)
    .map((c, i) => `${i + 1}\n${time(c.start)} --> ${time(c.end)}\n${c.text}`)
    .join("\n\n") + "\n";
