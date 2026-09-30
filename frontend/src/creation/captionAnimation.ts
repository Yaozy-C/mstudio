import type { Caption } from "../model";
export const captionAnimations = {
  none: "无动画",
  pop: "弹入",
  typewriter: "逐字出现",
  highlight: "逐词高亮",
  rise: "上滑淡入",
  shake: "轻微抖动",
  glow: "呼吸发光",
};
export const animationPeriod = 1;
export type WordTiming = { text: string; start: number; end: number };
export function captionWords(c: Caption): WordTiming[] {
  if (c.words?.length && c.words.map((w) => w.text).join("") === c.text)
    return c.words;
  const tokens = Array.from(
    new Intl.Segmenter("zh", { granularity: "word" }).segment(c.text),
    (s) => s.segment,
  );
  return tokens.map((text, i) => ({
    text,
    start: (i / tokens.length) * (c.end - c.start),
    end: ((i + 1) / tokens.length) * (c.end - c.start),
  }));
}
export function animationState(c: Caption, time: number) {
  const duration = c.end - c.start;
  const t = Math.max(0, Math.min(duration, time));
  const progress = Math.min(1, t / Math.min(0.6, duration));
  const kind = c.animation ?? "none";
  const phase = ((t % animationPeriod) / animationPeriod) * Math.PI * 2;
  const state = {
    scale: 1,
    opacity: 1,
    dx: 0,
    dy: 0,
    glow: 0,
    visible: c.text.length,
    from: -1,
    to: -1,
  };
  if (kind === "pop") {
    state.scale =
      progress === 1
        ? 1
        : 1 - Math.exp(-7 * progress) * Math.cos(10 * progress);
    state.opacity = Math.min(1, progress * 5);
  }
  if (kind === "rise") {
    state.dy = (1 - progress) ** 3 * 0.8;
    state.opacity = 1 - (1 - progress) ** 3;
  }
  if (kind === "shake") {
    state.dx = Math.sin(phase * 3) * 0.06;
    state.dy = Math.sin(phase * 2) * 0.03;
  }
  if (kind === "glow") state.glow = 0.08 + (1 - Math.cos(phase)) * 0.16;
  if (kind === "typewriter") {
    const chars = Array.from(
      new Intl.Segmenter("zh", { granularity: "grapheme" }).segment(c.text),
    );
    const count = Math.min(
      chars.length,
      Math.floor((t / (duration * 0.85)) * chars.length + 1e-8) + 1,
    );
    state.visible = count < chars.length ? chars[count].index : c.text.length;
  }
  if (kind === "highlight") {
    let offset = 0;
    for (const word of captionWords(c)) {
      if (t >= word.start && t < word.end) {
        state.from = offset;
        state.to = offset + word.text.length;
        break;
      }
      offset += word.text.length;
    }
  }
  return state;
}
export function animationPlan(c: Caption, fps: number) {
  const duration = c.end - c.start;
  const loop = c.animation === "shake" || c.animation === "glow";
  const length = loop
    ? Math.min(animationPeriod, duration)
    : c.animation === "pop" || c.animation === "rise"
      ? Math.min(0.6 + 1 / fps, duration)
      : duration;
  const frames: { time: number; duration: number }[] = [];
  let lastKey = "";
  const total = Math.ceil(length * fps);
  const boundaries = new Set<number>([0, total]);
  if (c.animation === "typewriter") {
    const count = Array.from(
      new Intl.Segmenter("zh", { granularity: "grapheme" }).segment(c.text),
    ).length;
    for (let i = 1; i < count; i++)
      boundaries.add(Math.ceil((i / count) * duration * 0.85 * fps - 1e-8));
  } else if (c.animation === "highlight") {
    for (const word of captionWords(c))
      for (const t of [word.start, word.end])
        boundaries.add(Math.min(total, Math.ceil(t * fps - 1e-8)));
  } else for (let i = 1; i < total; i++) boundaries.add(i);
  const points = [...boundaries].sort((a, b) => a - b);
  for (let i = 0; i < points.length - 1; i++) {
    const time = points[i] / fps;
    const span = (points[i + 1] - points[i]) / fps;
    const key = JSON.stringify(animationState(c, time));
    if (key === lastKey) frames[frames.length - 1].duration += span;
    else {
      frames.push({ time, duration: span });
      lastKey = key;
    }
  }
  return { frames, loop };
}
export function validAnimation(c: Caption) {
  return (
    (c.animation === undefined ||
      Object.hasOwn(captionAnimations, c.animation)) &&
    (c.highlightColor === undefined ||
      /^#[0-9a-f]{6}$/i.test(c.highlightColor)) &&
    (c.words === undefined ||
      (c.words.length <= 1000 &&
        c.words.map((w) => w.text).join("") === c.text &&
        c.words.every(
          (w, i) =>
            w.text.length > 0 &&
            Number.isFinite(w.start) &&
            Number.isFinite(w.end) &&
            w.start >= 0 &&
            w.end > w.start &&
            w.end <= c.end - c.start + 0.000001 &&
            (i === 0 || w.start >= c.words![i - 1].end),
        )))
  );
}
