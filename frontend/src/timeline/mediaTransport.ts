import type { PlaybackClock } from "./clock";
import type { IndexedClip } from "./geometry";
export type TransportStats = {
  seeks: number;
  waiting: number;
  playCalls: number;
};
/** A media element owns time. Never chase a wall clock by seeking during playback. */
export function bindVideo(
  el: HTMLMediaElement,
  entry: IndexedClip,
  clock: PlaybackClock,
  onError: (message: string) => void,
  stats: TransportStats,
  master = true,
) {
  let disposed = false,
    positioned = false,
    requested = false,
    revision = -1,
    correctedAt = 0;
  el.playbackRate = entry.clip.speed;
  el.volume = entry.clip.volume;
  const position = () => {
    if (el.readyState < 1) return;
    const t =
      entry.clip.trimIn +
      Math.max(0, clock.getSnapshot().time - entry.start) * entry.clip.speed;
    const target = Math.min(entry.clip.trimOut, Math.max(entry.clip.trimIn, t));
    if (Math.abs(el.currentTime - target) > 0.001) {
      el.currentTime = target;
      stats.seeks++;
    }
    positioned = true;
    revision = clock.seekRevision;
  };
  const sync = () => {
    if (disposed) return;
    if (!positioned || revision !== clock.seekRevision) position();
    const state = clock.getSnapshot();
    if (master && state.playing && (!positioned || el.readyState < 2))
      clock.setBuffering(true);
    const local = Math.max(0, state.time - entry.start);
    const gain = Math.min(
      1,
      entry.clip.fadeIn ? local / entry.clip.fadeIn : 1,
      entry.clip.fadeOut ? (entry.end - state.time) / entry.clip.fadeOut : 1,
    );
    const volume = Math.max(0, Math.min(1, entry.clip.volume * gain));
    if (Math.abs(el.volume - volume) > 0.001) el.volume = volume;
    // Small audio/video drift should not flush the decoder with repeated seeks.
    if (
      !master &&
      state.playing &&
      !clock.buffering &&
      performance.now() - correctedAt > 1000 &&
      el.readyState >= 2
    ) {
      correctedAt = performance.now();
      const drift =
        entry.clip.trimIn + local * entry.clip.speed - el.currentTime;
      if (Math.abs(drift) > 0.5) {
        position();
        el.playbackRate = entry.clip.speed;
      } else {
        const correction =
          Math.abs(drift) < 0.04
            ? 1
            : 1 + Math.max(-0.08, Math.min(0.08, drift * 0.5));
        const rate = entry.clip.speed * correction;
        if (Math.abs(el.playbackRate - rate) > 0.001) el.playbackRate = rate;
      }
    }
    if (state.playing && (master || !clock.buffering)) {
      if (positioned && !requested) {
        requested = true;
        stats.playCalls++;
        void el.play().catch((e: DOMException) => {
          if (disposed) return;
          requested = false;
          if (e.name !== "AbortError") {
            onError("媒体播放失败，请尝试流畅预览或检查素材");
            clock.pause();
          }
        });
      }
    } else {
      requested = false;
      if (!el.paused) el.pause();
      if (el.playbackRate !== entry.clip.speed)
        el.playbackRate = entry.clip.speed;
    }
  };
  const source = () => {
    if (!positioned || el.seeking || el.readyState < 2)
      return clock.getSnapshot().time;
    if (el.ended) return entry.end;
    return Math.min(
      entry.end,
      entry.start +
        Math.max(0, el.currentTime - entry.clip.trimIn) / entry.clip.speed,
    );
  };
  const waiting = () => {
    if (clock.getSnapshot().playing) {
      stats.waiting++;
      if (master) clock.setBuffering(true);
    }
  };
  const playing = () => {
    if (master) clock.setBuffering(false);
  };
  if (master) clock.setSource(source);
  el.addEventListener("playing", playing);
  el.addEventListener("loadedmetadata", sync);
  el.addEventListener("canplay", sync);
  el.addEventListener("waiting", waiting);
  sync();
  const off = clock.subscribe(sync);
  return () => {
    disposed = true;
    off();
    clock.clearSource(source);
    el.removeEventListener("loadedmetadata", sync);
    el.removeEventListener("canplay", sync);
    el.removeEventListener("waiting", waiting);
    el.removeEventListener("playing", playing);
    if (master) clock.setBuffering(false);
    if (!el.paused) el.pause();
  };
}
