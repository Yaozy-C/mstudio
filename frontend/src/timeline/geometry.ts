import { duration, type Clip } from "../model";
export const clampZoom = (zoom: number, fps: number) =>
  Math.max(4, Math.min(fps * 32, zoom));
export const frameTime = (time: number, fps: number) => {
  const frames = Math.max(0, Math.round(time * fps));
  return `${Math.floor(frames / fps / 60)
    .toString()
    .padStart(
      2,
      "0",
    )}:${(Math.floor(frames / fps) % 60).toString().padStart(2, "0")}:${(frames % fps).toString().padStart(2, "0")}`;
};
export function rulerStep(zoom: number, fps: number) {
  return (
    [
      1,
      2,
      5,
      10,
      Math.round(fps / 2),
      fps,
      fps * 2,
      fps * 5,
      fps * 10,
      fps * 30,
      fps * 60,
      fps * 300,
    ]
      .sort((a, b) => a - b)
      .find((f) => (f / fps) * zoom >= 72) || fps * 600
  );
}
export function ticks(left: number, width: number, zoom: number, fps: number) {
  const step = rulerStep(zoom, fps);
  const first = Math.floor(((left / zoom) * fps) / step) * step;
  const result: number[] = [];
  for (
    let f = Math.max(0, first);
    f <= ((left + width) / zoom) * fps + step;
    f += step
  )
    result.push(f);
  return result;
}
export function clipIndex(clips: Clip[]) {
  let time = 0;
  return clips.map((clip) => {
    const start = clip.start;
    time = start + duration(clip);
    return { clip, start, end: time };
  });
}
export type IndexedClip = ReturnType<typeof clipIndex>[number];
export function visibleClips(index: IndexedClip[], from: number, to: number) {
  let lo = 0,
    hi = index.length;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    if (index[mid].end < from) lo = mid + 1;
    else hi = mid;
  }
  const result: IndexedClip[] = [];
  for (let i = lo; i < index.length && index[i].start <= to; i++)
    result.push(index[i]);
  return result;
}
