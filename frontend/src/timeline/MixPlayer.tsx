import { useLayoutEffect, useMemo, useRef } from "react";
import { mediaUrl } from "../bridge";
import type { PlaybackClock } from "./clock";
import { bindVideo, type TransportStats } from "./mediaTransport";
export function MixPlayer({
  path,
  total,
  clock,
  stats,
  onError,
}: {
  path: string;
  total: number;
  clock: PlaybackClock;
  stats: TransportStats;
  onError: (s: string) => void;
}) {
  const ref = useRef<HTMLAudioElement>(null);
  const entry = useMemo(
    () => ({
      start: 0,
      end: total,
      clip: {
        id: "preview-mix",
        assetId: "mix",
        trimIn: 0,
        trimOut: total,
        speed: 1,
        volume: 1,
        start: 0,
        trackId: "v1",
      },
    }),
    [total],
  );
  useLayoutEffect(() => {
    if (!ref.current) return;
    return bindVideo(ref.current, entry, clock, onError, stats, true);
  }, [path, entry, clock, onError, stats]);
  return <audio ref={ref} src={mediaUrl(path)} preload="auto" />;
}
