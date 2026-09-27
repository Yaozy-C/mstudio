import { t, useLanguage } from "../i18n";
import { useEffect, useState } from "react";
import type { TransportStats } from "./mediaTransport";
export function PlaybackDiagnostics({
  element,
  stats,
}: {
  element: HTMLVideoElement | null;
  stats: TransportStats;
}) {
  useLanguage();
  const [text, setText] = useState(t("等待视频播放"));
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
        t(
          "{v0} 层视频 · 解码 {v1} 帧 · 丢帧 {v2} · 定位 {v3} 次 · 缓冲 {v4} 次 · play {v5} 次",
          {
            v0: active.length,
            v1: decoded,
            v2: dropped,
            v3: stats.seeks,
            v4: stats.waiting,
            v5: stats.playCalls,
          },
        ),
      );
    };
    update();
    const timer = setInterval(update, 500);
    return () => clearInterval(timer);
  }, [element, stats]);
  return <output className="playback-diagnostics">{text}</output>;
}
