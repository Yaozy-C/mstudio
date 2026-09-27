import { t, useLanguage } from "../i18n";
import { useEffect, useRef } from "react";
import type { PlaybackClock } from "./clock";
import { frameTime } from "./geometry";
export function ClockReadout({ clock }: { clock: PlaybackClock }) {
  useLanguage();
  const ref = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    const update = () => {
      if (ref.current)
        ref.current.textContent = frameTime(
          clock.getSnapshot().time,
          clock.fps,
        );
    };
    update();
    return clock.subscribe(update);
  }, [clock]);
  return (
    <span ref={ref} className="clock-readout" aria-label={t("播放时间")}>
      00:00:00
    </span>
  );
}
export function Playhead({
  clock,
  zoom,
}: {
  clock: PlaybackClock;
  zoom: number;
}) {
  useLanguage();
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const update = () => {
      if (ref.current)
        ref.current.style.transform = `translateX(${clock.getSnapshot().time * zoom}px)`;
    };
    update();
    return clock.subscribe(update);
  }, [clock, zoom]);
  return (
    <div ref={ref} className="playhead" style={{ left: 0 }}>
      <span />
    </div>
  );
}
