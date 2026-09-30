import { useEffect, useRef } from "react";
import type { Caption } from "../model";
import type { PlaybackClock } from "./clock";
import { captionImage } from "../creation/captions";
export function AnimatedCaption({
  caption,
  width,
  height,
  fps,
  clock,
}: {
  caption: Caption;
  width: number;
  height: number;
  fps: number;
  clock: PlaybackClock;
}) {
  const image = useRef<HTMLImageElement>(null);
  useEffect(() => {
    let last = -1;
    const sync = () => {
      const frame = Math.max(
        0,
        Math.floor((clock.getSnapshot().time - caption.start) * fps + 1e-7),
      );
      if (last === frame) return;
      last = frame;
      if (image.current)
        image.current.src = captionImage(
          caption,
          width,
          height,
          caption.animation && caption.animation !== "none"
            ? frame / fps
            : undefined,
        );
    };
    sync();
    return clock.subscribe(sync);
  }, [caption, width, height, fps, clock]);
  return <img ref={image} className="caption-overlay" alt={caption.text} />;
}
