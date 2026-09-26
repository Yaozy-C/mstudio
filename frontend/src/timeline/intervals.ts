import { duration, type Clip } from "../model";
import type { IndexedClip } from "./geometry";
export type Interval = IndexedClip & { maxEnd: number };
/** Balanced interval tree stored in a sorted array; maxEnd belongs to each subtree. */
export function intervals(clips: Clip[]): Interval[] {
  const index = clips
    .map((clip) => ({
      clip,
      start: clip.start ?? 0,
      end: (clip.start ?? 0) + duration(clip),
      maxEnd: 0,
    }))
    .sort((a, b) => a.start - b.start);
  const build = (lo: number, hi: number): number => {
    if (lo >= hi) return 0;
    const mid = (lo + hi) >>> 1;
    return (index[mid].maxEnd = Math.max(
      index[mid].end,
      build(lo, mid),
      build(mid + 1, hi),
    ));
  };
  build(0, index.length);
  return index;
}
export function inRange(
  index: Interval[],
  from: number,
  to: number,
): Interval[] {
  const found: Interval[] = [];
  const walk = (lo: number, hi: number) => {
    if (lo >= hi || index[lo].start > to) return;
    const mid = (lo + hi) >>> 1,
      item = index[mid];
    if (item.maxEnd <= from) return;
    walk(lo, mid);
    if (item.start <= to && item.end > from) found.push(item);
    if (item.start <= to) walk(mid + 1, hi);
  };
  walk(0, index.length);
  return found;
}
export const activeAt = (index: Interval[], time: number) =>
  inRange(index, time, time);
export function upcoming(index: Interval[], time: number, horizon = 2) {
  let lo = 0,
    hi = index.length;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if (index[mid].start <= time) lo = mid + 1;
    else hi = mid;
  }
  const result = new Map<string, Interval>();
  for (let i = lo; i < index.length && index[i].start <= time + horizon; i++) {
    const e = index[i];
    if (!result.has(e.clip.trackId!)) result.set(e.clip.trackId!, e);
  }
  return [...result.values()];
}
