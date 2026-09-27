import { t, useLanguage } from "../i18n";
import { useSyncExternalStore } from "react";
import { Play, Pause, SkipBack } from "@phosphor-icons/react";
import type { PlaybackClock } from "./clock";
export function TimelineTransport({
  clock,
  onPlay,
}: {
  clock: PlaybackClock;
  onPlay: () => void;
}) {
  useLanguage();
  const playing = useSyncExternalStore(
    clock.subscribe,
    () => clock.getSnapshot().playing,
  );
  return (
    <div className="timeline-transport">
      <button
        className="icon-button"
        aria-label={playing ? t("暂停") : t("播放成片")}
        onClick={onPlay}
      >
        {playing ? <Pause weight="fill" /> : <Play weight="fill" />}
      </button>
      <button
        className="icon-button"
        aria-label={t("返回起点")}
        onClick={() => {
          clock.pause();
          clock.seek(0);
        }}
      >
        <SkipBack />
      </button>
    </div>
  );
}
