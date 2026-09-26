import { useEffect, useRef, useState, type PointerEvent } from "react";
import {
  compactPanel,
  fitPanel,
  resizePanel,
  type Bounds,
  type ResizeAxis,
} from "./panelGeometry";
const screen = () => ({ width: window.innerWidth, height: window.innerHeight });
export function usePanelBounds(
  id: string,
  initial: Bounds,
  collapsed: boolean,
  bottomInset = 0,
) {
  const available = () => ({
    ...screen(),
    height: window.innerHeight - bottomInset,
  });
  const [bounds, setBounds] = useState<Bounds>(() => {
    try {
      const value = JSON.parse(
        localStorage.getItem(`mstudio-panel:${id}`) || "null",
      );
      return value &&
        [value.x, value.y, value.width, value.height].every(Number.isFinite)
        ? value
        : initial;
    } catch {
      return initial;
    }
  });
  const [viewport, setViewport] = useState(screen);
  const ref = useRef<HTMLElement>(null);
  const drag = useRef<{
    x: number;
    y: number;
    start: Bounds;
    next: Bounds;
  } | null>(null);
  const expanded = fitPanel(bounds, {
    ...viewport,
    height: viewport.height - bottomInset,
  });
  const display = collapsed ? compactPanel(expanded, id === "agent") : expanded;
  useEffect(() => {
    const resize = () => setViewport(screen());
    window.addEventListener("resize", resize);
    return () => window.removeEventListener("resize", resize);
  }, []);
  const persist = (next: Bounds) => {
    setBounds(next);
    localStorage.setItem(`mstudio-panel:${id}`, JSON.stringify(next));
  };
  const paint = (next: Bounds) => {
    const box = collapsed ? compactPanel(next, id === "agent") : next;
    if (ref.current)
      Object.assign(ref.current.style, {
        left: `${box.x}px`,
        top: `${box.y}px`,
        width: `${box.width}px`,
        height: `${box.height}px`,
      });
  };
  const finish = (e: PointerEvent<HTMLElement>) => {
    const current = drag.current;
    if (!current) return;
    drag.current = null;
    if (e.currentTarget.hasPointerCapture(e.pointerId))
      e.currentTarget.releasePointerCapture(e.pointerId);
    persist(current.next);
  };
  const handlers = (axis: ResizeAxis | "move") => ({
    onPointerDown: (e: PointerEvent<HTMLElement>) => {
      if (
        e.button !== 0 ||
        (axis === "move" && (e.target as HTMLElement).closest("button"))
      )
        return;
      e.preventDefault();
      if (axis !== "move") e.currentTarget.focus({ preventScroll: true });
      e.currentTarget.setPointerCapture(e.pointerId);
      const start = fitPanel(bounds, available());
      drag.current = { x: e.clientX, y: e.clientY, start, next: start };
    },
    onPointerMove: (e: PointerEvent<HTMLElement>) => {
      const d = drag.current;
      if (!d || !e.currentTarget.hasPointerCapture(e.pointerId)) return;
      const dx = e.clientX - d.x,
        dy = e.clientY - d.y;
      d.next =
        axis === "move"
          ? fitPanel(
              { ...d.start, x: d.start.x + dx, y: d.start.y + dy },
              available(),
            )
          : resizePanel(d.start, dx, dy, axis, available());
      // Dragging updates only geometry; the chat/canvas need not render per pointer event.
      paint(d.next);
    },
    onPointerUp: finish,
    onPointerCancel: finish,
    onLostPointerCapture: finish,
  });
  return {
    ref,
    expanded,
    handlers,
    style: {
      left: display.x,
      top: display.y,
      width: display.width,
      height: display.height,
    },
    resizeBy: (dx: number, dy: number, axis: ResizeAxis) =>
      persist(resizePanel(expanded, dx, dy, axis, available())),
  };
}
