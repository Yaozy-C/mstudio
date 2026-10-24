/** Codex image requests need an explicit size; pick a valid 16-multiple frame. */
const STANDARD = [
  { ratio: 1, size: "1024x1024" },
  { ratio: 16 / 9, size: "2048x1152" },
  { ratio: 9 / 16, size: "1152x2048" },
  { ratio: 4 / 3, size: "2048x1536" },
  { ratio: 3 / 4, size: "1536x2048" },
  { ratio: 3 / 2, size: "1536x1024" },
  { ratio: 2 / 3, size: "1024x1536" },
  { ratio: 21 / 9, size: "2016x864" },
  { ratio: 4 / 5, size: "1024x1280" },
  { ratio: 5 / 4, size: "1280x1024" },
];
const MAX_EDGE = 3840;
const MIN_PIXELS = 655_360;
const DEFAULT_SIZE = "2048x1152";

export function codexImageSize(width: number, height: number): string {
  if (
    !Number.isFinite(width) ||
    !Number.isFinite(height) ||
    width <= 0 ||
    height <= 0
  )
    return DEFAULT_SIZE;
  const ratio = width / height;
  if (ratio < 1 / 3 || ratio > 3) return DEFAULT_SIZE;

  let best = STANDARD[0];
  for (const candidate of STANDARD) {
    if (
      Math.abs(Math.log(ratio / candidate.ratio)) <
      Math.abs(Math.log(ratio / best.ratio))
    )
      best = candidate;
  }
  if (Math.abs(Math.log(ratio / best.ratio)) <= 0.06) return best.size;

  const long = 2048;
  let short = Math.max(16, Math.round(long / Math.max(ratio, 1) / 16) * 16);
  let w = ratio >= 1 ? long : short;
  let h = ratio >= 1 ? short : long;
  const pixels = w * h;
  if (pixels < MIN_PIXELS) {
    const scale = Math.sqrt(MIN_PIXELS / pixels);
    w = Math.round((w * scale) / 16) * 16;
    h = Math.round((h * scale) / 16) * 16;
  }
  return `${Math.min(w, MAX_EDGE)}x${Math.min(h, MAX_EDGE)}`;
}
