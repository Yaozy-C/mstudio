import { uid, type Caption } from "../model";
import { captionFonts, captionKey } from "./captionStyle";
const images = new Map<string, string>();
/** Identical raster used by preview and native export; no platform subtitle-font mismatch. */
export function captionImage(
  input: string | Caption,
  width: number,
  height: number,
) {
  const caption: Caption =
    typeof input === "string"
      ? { id: "", start: 0, end: 1, text: input }
      : input;
  const text = caption.text;
  const key = `${width}x${height}:${captionKey(caption)}`;
  const cached = images.get(key);
  if (cached) return cached;
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext("2d")!;
  let size = Math.round(Math.min(width, height) * (caption.fontSize ?? 0.048)),
    lines: string[] = [];
  const wrap = () => {
    ctx.font = `600 ${size}px ${captionFonts[caption.font ?? "sans"].family}`;
    lines = [];
    let line = "";
    for (const ch of text) {
      if (
        ch === "\n" ||
        (line && ctx.measureText(line + ch).width > width * 0.86)
      ) {
        lines.push(line);
        line = ch === "\n" ? "" : ch;
      } else line += ch;
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
  ctx.fillStyle = "rgba(0,0,0,.65)";
  if (caption.background !== false)
    ctx.fillRect(
      x - boxWidth / 2,
      y - size * 0.2,
      boxWidth,
      lines.length * lineHeight + size * 0.4,
    );
  ctx.fillStyle = caption.color ?? "#ffffff";
  ctx.shadowColor = "rgba(0,0,0,.8)";
  ctx.shadowBlur = caption.background === false ? size * 0.08 : 0;
  lines.forEach((l, i) => ctx.fillText(l, x, y + i * lineHeight));
  const url = canvas.toDataURL("image/png");
  if (images.size >= 32) images.delete(images.keys().next().value!);
  images.set(key, url);
  return url;
}
export function validCaption(c: Caption) {
  return (
    Number.isFinite(c.start) &&
    Number.isFinite(c.end) &&
    c.start >= 0 &&
    c.end > c.start &&
    c.end <= 86400 &&
    c.text.trim().length > 0 &&
    c.text.length <= 1000 &&
    (c.font === undefined || c.font in captionFonts) &&
    (c.color === undefined || /^#[0-9a-f]{6}$/i.test(c.color)) &&
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
