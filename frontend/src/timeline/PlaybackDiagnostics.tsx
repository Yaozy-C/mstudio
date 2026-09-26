import { useEffect, useState } from "react";
import type { TransportStats } from "./mediaTransport";
export function PlaybackDiagnostics({
  element,
  stats,
}: {
  element: HTMLVideoElement | null;
  stats: TransportStats;
}) {
  const [text, setText] = useState("等待视频播放");
  useEffect(() => {
    const update = () => {
      const videos = Array.from(
        document.querySelectorAll<HTMLVideoElement>(".preview-frame video"),
      );
      const active = videos.filter((v) => v.style.display !== "none");
      const quality = videos.map((v) => v.getVideoPlaybackQuality?.());
      const decoded = quality.reduce(
        (n, q) => n + (q?.totalVideoFrames ?? 0),
        0,
      );
      const dropped = quality.reduce(
        (n, q) => n + (q?.droppedVideoFrames ?? 0),
        0,
      );
      setText(
        `${active.length} 层视频 · 解码 ${decoded} 帧 · 丢帧 ${dropped} · 定位 ${stats.seeks} 次 · 缓冲 ${stats.waiting} 次 · play ${stats.playCalls} 次`,
      );
    };
    update();
    const timer = setInterval(update, 500);
    return () => clearInterval(timer);
  }, [element, stats]);
  return <output className="playback-diagnostics">{text}</output>;
}
