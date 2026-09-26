import { useEffect, useRef, useState } from "react";
import type { PlaybackClock } from "./clock";
import { clampZoom } from "./geometry";
export function useTimelineViewport(clock: PlaybackClock) {
  const ref = useRef<HTMLDivElement>(null);
  const labels = useRef<HTMLDivElement>(null);
  const [zoom, setZoom] = useState(() =>
    clampZoom(
      Number(localStorage.getItem("mstudio-time-zoom")) || 64,
      clock.fps,
    ),
  );
  const [viewport, setViewport] = useState({ left: 0, width: 900 });
  const zoomRef = useRef(zoom);
  zoomRef.current = zoom;
  const changeZoom = (factor: number, clientX?: number) => {
    const el = ref.current;
    if (!el) return;
    const offset =
      clientX === undefined
        ? Math.max(
            0,
            Math.min(
              el.clientWidth,
              clock.getSnapshot().time * zoomRef.current - el.scrollLeft,
            ),
          )
        : clientX - el.getBoundingClientRect().left;
    const time = (el.scrollLeft + offset) / zoomRef.current;
    const next = clampZoom(zoomRef.current * factor, clock.fps);
    zoomRef.current = next;
    setZoom(next);
    localStorage.setItem("mstudio-time-zoom", String(next));
    requestAnimationFrame(() => {
      el.scrollLeft = Math.max(0, time * next - offset);
      setViewport({ left: el.scrollLeft, width: el.clientWidth });
    });
  };
  const changeRef = useRef(changeZoom);
  changeRef.current = changeZoom;
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const update = () =>
      setViewport({ left: el.scrollLeft, width: el.clientWidth });
    const observer = new ResizeObserver(update);
    observer.observe(el);
    update();
    const wheel = (event: WheelEvent) => {
      if (event.ctrlKey || event.metaKey) {
        event.preventDefault();
        changeRef.current(Math.exp(-event.deltaY * 0.01), event.clientX);
      }
    };
    const labelWheel = (event: WheelEvent) => {
      event.preventDefault();
      if (event.ctrlKey || event.metaKey) {
        changeRef.current(Math.exp(-event.deltaY * 0.01));
        return;
      }
      const unit =
        event.deltaMode === 1
          ? 16
          : event.deltaMode === 2
            ? el.clientHeight
            : 1;
      el.scrollTop += event.shiftKey ? 0 : event.deltaY * unit;
      el.scrollLeft +=
        (event.deltaX + (event.shiftKey ? event.deltaY : 0)) * unit;
    };
    const labelElement = labels.current;
    labelElement?.addEventListener("wheel", labelWheel, { passive: false });
    const zoomEvent = (event: Event) =>
      changeRef.current((event as CustomEvent<number>).detail);
    el.addEventListener("wheel", wheel, { passive: false });
    window.addEventListener("timeline-zoom", zoomEvent);
    let revision = clock.seekRevision;
    const unsubscribe = clock.subscribe(() => {
      const s = clock.getSnapshot();
      const seeking = revision !== clock.seekRevision;
      revision = clock.seekRevision;
      if (!s.playing && !seeking) return;
      const x = s.time * zoomRef.current;
      if (x > el.scrollLeft + el.clientWidth - 32 || x < el.scrollLeft)
        el.scrollLeft = Math.max(0, x - 40);
    });
    return () => {
      observer.disconnect();
      el.removeEventListener("wheel", wheel);
      labelElement?.removeEventListener("wheel", labelWheel);
      window.removeEventListener("timeline-zoom", zoomEvent);
      unsubscribe();
    };
  }, [clock]);
  return {
    ref,
    labels,
    zoom,
    viewport,
    changeZoom,
    onScroll: () => {
      if (ref.current)
        setViewport({
          left: ref.current.scrollLeft,
          width: ref.current.clientWidth,
        });
    },
  };
}
