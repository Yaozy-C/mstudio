import { useLayoutEffect, type RefObject } from "react";

/** Animate navigation only; edits and background updates keep their position. */
export function useContentMotion(
  ref: RefObject<HTMLElement | null>,
  page: string,
  depth = true,
) {
  useLayoutEffect(() => {
    const element = ref.current;
    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");
    if (!element?.animate || reduced.matches) return;
    const style = getComputedStyle(element);
    const animation = element.animate(
      depth
        ? [
            { opacity: 0.72, filter: "blur(2px)" },
            { opacity: 1, filter: "blur(0)" },
          ]
        : [{ opacity: 0.8 }, { opacity: 1 }],
      {
        duration: parseFloat(style.getPropertyValue("--motion-page")) || 200,
        easing: style.getPropertyValue("--motion-ease").trim(),
      },
    );
    const preferenceChanged = () => {
      if (reduced.matches) animation.cancel();
    };
    reduced.addEventListener("change", preferenceChanged);
    return () => {
      animation.cancel();
      reduced.removeEventListener("change", preferenceChanged);
    };
  }, [ref, page, depth]);
}
