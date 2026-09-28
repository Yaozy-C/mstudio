import { t, useLanguage } from "../i18n";
import { memo, useLayoutEffect, useMemo, useRef } from "react";
import { visualCss } from "./visualCss";
import { mediaUrl } from "../bridge";
import { MissingAsset } from "../workspace/MissingAsset";
import type { Asset, Track } from "../model";
import type { IndexedClip } from "./geometry";
import type { PlaybackClock } from "./clock";
import { bindVideo, type TransportStats } from "./mediaTransport";
export const MediaLayer = memo(function MediaLayer({
  entry,
  asset,
  track,
  active,
  master,
  clock,
  stats,
  onError,
  onElement,
}: {
  entry: IndexedClip;
  asset: Asset;
  track: Track;
  active: boolean;
  master: boolean;
  clock: PlaybackClock;
  stats: TransportStats;
  onError: (s: string) => void;
  onElement: (e: HTMLVideoElement | null) => void;
}) {
  useLanguage();
  const ref = useRef<HTMLVideoElement | HTMLAudioElement | null>(null);
  const src = mediaUrl(asset.path);
  const bound = useMemo(
    () =>
      track.muted ? { ...entry, clip: { ...entry.clip, volume: 0 } } : entry,
    [entry, track.muted],
  );
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el || asset.missing) return;
    el.muted = !active || !asset.hasAudio || !!track.muted;
    if (!active) {
      const prepare = () => {
        if (
          entry.start > clock.getSnapshot().time &&
          el.readyState >= 1 &&
          Math.abs(el.currentTime - entry.clip.trimIn) > 0.01
        )
          el.currentTime = entry.clip.trimIn;
      };
      el.pause();
      prepare();
      el.addEventListener("loadedmetadata", prepare);
      return () => el.removeEventListener("loadedmetadata", prepare);
    }
    if (master) onElement(el instanceof HTMLVideoElement ? el : null);
    return bindVideo(el, bound, clock, onError, stats, master);
  }, [
    active,
    master,
    bound,
    src,
    clock,
    stats,
    onError,
    onElement,
    entry.clip.trimIn,
    entry.start,
    asset.hasAudio,
    track.muted,
    asset.missing,
  ]);
  const c = entry.clip;
  const visual = track.kind === "video" && !track.hidden;
  const style = {
    ...visualCss(c.visual),
    left: `${(c.x ?? 0.5) * 100}%`,
    top: `${(c.y ?? 0.5) * 100}%`,
    width: `${(c.scale ?? 1) * 100}%`,
    height: `${(c.scale ?? 1) * 100}%`,
    opacity: c.opacity ?? 1,
    display: active && visual ? undefined : "none",
  };
  const error = () => {
    if (active) {
      onError(t("无法播放 {v0}", { v0: asset.name }));
      clock.pause();
    }
  };
  if (asset.missing)
    return active && visual ? (
      <div className="composition-layer" style={style}>
        <MissingAsset asset={asset} />
      </div>
    ) : null;
  if (asset.kind === "image")
    return <img className="composition-layer" src={src} style={style} />;
  if (asset.kind === "audio" || !visual)
    return (
      <audio
        ref={ref as React.RefObject<HTMLAudioElement>}
        src={src}
        preload="auto"
        onError={error}
      />
    );
  return (
    <video
      ref={ref as React.RefObject<HTMLVideoElement>}
      className="composition-layer"
      style={style}
      src={src}
      preload="auto"
      playsInline
      muted={!active || !asset.hasAudio || !!track.muted}
      onError={error}
    />
  );
});
