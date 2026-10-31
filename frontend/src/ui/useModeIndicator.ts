import { useLayoutEffect, type RefObject } from "react";

/** Move only the decoration; the existing navigation buttons stay untouched. */
export function useModeIndicator(
  ref: RefObject<HTMLElement | null>,
  selected: string,
) {
  useLayoutEffect(() => {
    const nav = ref.current;
    if (!nav) return;
    const measure = () => {
      const button = nav.querySelector<HTMLElement>('[aria-pressed="true"]');
      if (!button) {
        delete nav.dataset.indicator;
        return;
      }
      const container = nav.getBoundingClientRect();
      const bounds = button.getBoundingClientRect();
      nav.style.setProperty(
        "--mode-left",
        `${bounds.left - container.left + 5}px`,
      );
      nav.style.setProperty(
        "--mode-width",
        `${Math.max(0, bounds.width - 10)}px`,
      );
      nav.dataset.indicator ??= "initial";
    };
    measure();
    const frame = requestAnimationFrame(() => {
      if (nav.querySelector('[aria-pressed="true"]'))
        nav.dataset.indicator = "ready";
    });
    const resize = new ResizeObserver(measure);
    resize.observe(nav);
    nav.querySelectorAll("button").forEach((button) => resize.observe(button));
    return () => {
      cancelAnimationFrame(frame);
      resize.disconnect();
    };
  }, [ref, selected]);
}
