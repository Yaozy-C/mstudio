import { useEffect, useRef, useState, type CSSProperties } from "react";

type Side = "media" | "agent";
const defaults = { media: 240, agent: 420 };
const minimum = { media: 180, agent: 320 };
const maximum = { media: 480, agent: 800 };
const key = "mstudio-sidebar-widths";
function initial() {
  try {
    const saved = JSON.parse(localStorage.getItem(key) || "{}");
    return Object.fromEntries(
      (["media", "agent"] as const).map((side) => [
        side,
        Number.isFinite(saved[side])
          ? Math.max(minimum[side], Math.min(maximum[side], saved[side]))
          : defaults[side],
      ]),
    ) as typeof defaults;
  } catch {
    return defaults;
  }
}
export function useSidebarWidths(visible: Record<Side, boolean>) {
  const [widths, setWidths] = useState(initial);
  const [viewport, setViewport] = useState(window.innerWidth);
  const [dragging, setDragging] = useState<Side | null>(null);
  const drag = useRef<{ side: Side; x: number; width: number } | null>(null);
  useEffect(() => {
    const resize = () => setViewport(window.innerWidth);
    window.addEventListener("resize", resize);
    return () => window.removeEventListener("resize", resize);
  }, []);
  useEffect(() => {
    try {
      localStorage.setItem(key, JSON.stringify(widths));
    } catch {
      /* session-only */
    }
  }, [widths]);
  const space = Math.max(1000, viewport) - 360;
  const left = visible.media
    ? Math.min(widths.media, space - (visible.agent ? minimum.agent : 0))
    : 0;
  const right = visible.agent ? Math.min(widths.agent, space - left) : 0;
  const shown = { media: left, agent: right };
  function limit(side: Side) {
    return Math.min(
      maximum[side],
      space - shown[side === "media" ? "agent" : "media"],
    );
  }
  function change(side: Side, value: number) {
    setWidths((w) => ({
      ...w,
      [side]: Math.max(minimum[side], Math.min(limit(side), value)),
    }));
  }
  const handles = (["media", "agent"] as const)
    .filter((s) => visible[s])
    .map((side) => (
      <div
        key={side}
        role="separator"
        tabIndex={0}
        className={`dock-resizer dock-resizer-${side}`}
        aria-label={side === "media" ? "调整素材栏宽度" : "调整助手栏宽度"}
        aria-orientation="vertical"
        aria-valuemin={minimum[side]}
        aria-valuemax={limit(side)}
        aria-valuenow={Math.round(shown[side])}
        title="拖动调整宽度 · 双击恢复默认"
        data-dragging={dragging === side}
        onPointerDown={(e) => {
          if (e.button !== 0) return;
          e.preventDefault();
          e.currentTarget.focus();
          e.currentTarget.setPointerCapture(e.pointerId);
          drag.current = { side, x: e.clientX, width: shown[side] };
          setDragging(side);
        }}
        onPointerMove={(e) => {
          const d = drag.current;
          if (d?.side === side)
            change(
              side,
              d.width + (e.clientX - d.x) * (side === "media" ? 1 : -1),
            );
        }}
        onPointerUp={(e) => {
          drag.current = null;
          setDragging(null);
          if (e.currentTarget.hasPointerCapture(e.pointerId))
            e.currentTarget.releasePointerCapture(e.pointerId);
        }}
        onLostPointerCapture={() => {
          drag.current = null;
          setDragging(null);
        }}
        onDoubleClick={() => change(side, defaults[side])}
        onKeyDown={(e) => {
          if (!["ArrowLeft", "ArrowRight", "Home"].includes(e.key)) return;
          e.preventDefault();
          if (e.key === "Home") change(side, defaults[side]);
          else
            change(
              side,
              shown[side] +
                (e.key === "ArrowRight" ? 1 : -1) *
                  (side === "media" ? 1 : -1) *
                  (e.shiftKey ? 40 : 10),
            );
        }}
      />
    ));
  return {
    style: {
      "--rail": `${left}px`,
      "--assistant": `${right}px`,
    } as CSSProperties,
    handles,
  };
}
