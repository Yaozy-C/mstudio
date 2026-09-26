import { useEffect, useRef, useState } from "react";
/** Fit the composition itself, so overlay coordinates are independent of panel aspect. */
export function useFrameSize(width: number, height: number) {
  const ref = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 1, height: 1 });
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const observer = new ResizeObserver(() => {
      const style = getComputedStyle(el);
      const w = Math.max(
        1,
        el.clientWidth -
          parseFloat(style.paddingLeft) -
          parseFloat(style.paddingRight),
      );
      const h = Math.max(
        1,
        el.clientHeight -
          parseFloat(style.paddingTop) -
          parseFloat(style.paddingBottom),
      );
      const scale = Math.min(w / width, h / height);
      setSize({ width: width * scale, height: height * scale });
    });
    observer.observe(el);
    return () => observer.disconnect();
  }, [width, height]);
  return { ref, size };
}
