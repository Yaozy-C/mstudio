import type { ScriptParagraph } from "./types";
export const paragraphDuration = (s: ScriptParagraph) =>
  Number.isFinite(s.duration) && s.duration! > 0 ? s.duration! : 5;
export const scriptDuration = (script: ScriptParagraph[]) =>
  Math.round(script.reduce((sum, s) => sum + paragraphDuration(s), 0) * 100) /
  100;
export const timeLabel = (seconds: number) => {
  const value = Math.round(seconds * 100) / 100;
  return `${Math.floor(value / 60)
    .toString()
    .padStart(2, "0")}:${(value % 60).toFixed(2).padStart(5, "0")}`;
};
// Preserve relative pacing; allocate integer centiseconds so the sum is exact.
export function retimeScript(script: ScriptParagraph[], seconds: number) {
  const units = Math.round(seconds * 100);
  if (
    !script.length ||
    !Number.isFinite(seconds) ||
    units < script.length ||
    seconds > 3600
  )
    return script;
  const total = scriptDuration(script);
  const available = units - script.length;
  const weights = script.map((s) => (available * paragraphDuration(s)) / total);
  const lengths = weights.map((w) => 1 + Math.floor(w));
  let remainder = units - lengths.reduce((a, b) => a + b, 0);
  const order = weights
    .map((w, i) => ({ i, fraction: w % 1 }))
    .sort((a, b) => b.fraction - a.fraction);
  for (const { i } of order) {
    if (remainder-- <= 0) break;
    lengths[i]++;
  }
  return script.map((s, i) => ({ ...s, duration: lengths[i] / 100 }));
}
