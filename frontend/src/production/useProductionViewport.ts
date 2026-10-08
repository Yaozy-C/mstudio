import { useEffect, useRef, useState } from "react";
import { fitView } from "../canvas/fit";
import { useViewport } from "../canvas/useViewport";
import type { Project } from "../model";
import type { ProductionController } from "./useProduction";
import { productionShots } from "./items";
export function useProductionViewport(
  project: Project,
  items: ProductionController["items"],
  focus: ProductionController["focus"],
  shots: ReturnType<typeof productionShots>,
) {
  const root = useRef<HTMLDivElement>(null);
  const { view, latest, update, hasSavedView } = useViewport(project);
  const setView = (
    value:
      Project["viewport"] | ((v: Project["viewport"]) => Project["viewport"]),
  ) => update(typeof value === "function" ? value(latest.current) : value);
  const [size, setSize] = useState({ width: 0, height: 0 });
  useEffect(() => {
    const element = root.current!;
    const observer = new ResizeObserver(([entry]) => {
      const { width, height } = entry.contentRect;
      setSize((old) =>
        old.width === width && old.height === height ? old : { width, height },
      );
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  function fit(all = false) {
    const bounds = root.current?.getBoundingClientRect();
    if (!bounds) return;
    const nodes = items
      .filter(
        (n) =>
          all ||
          (focus.itemKey
            ? n.key === focus.itemKey
            : n.ownerId === (focus.id ?? shots[0]?.id)),
      )
      .map((n) => ({
        id: n.key,
        x: n.x,
        y: n.y,
        width: n.width,
        height: n.height,
        title: n.title,
        kind: "asset" as const,
        text: n.text ?? "",
      }));
    if (!all && focus.itemKey) {
      if (!nodes.length) return;
      const panel = root.current
        ?.closest(".studio-stage")
        ?.querySelector(".production-task-panel")
        ?.getBoundingClientRect();
      const height =
        panel && panel.top < bounds.bottom && panel.bottom > bounds.top
          ? Math.max(1, panel.top - bounds.top)
          : bounds.height;
      setView(fitView(nodes, { width: bounds.width, height }));
      return;
    }
    const next = fitView(nodes, {
      width: bounds.width,
      height: bounds.height - 30,
    });
    if (!all && next.scale < 0.85) {
      next.scale = 0.85;
      next.x =
        40 -
        (nodes.length ? Math.min(...nodes.map((n) => n.x)) : 0) * next.scale;
    }
    setView({
      ...next,
      y: all
        ? next.y
        : 95 - Math.min(110, ...nodes.map((n) => n.y)) * next.scale,
    });
  }
  useEffect(() => {
    if (focus.tick || !hasSavedView) fit();
  }, [focus]);
  useEffect(() => {
    const el = root.current!;
    function wheel(e: WheelEvent) {
      e.preventDefault();
      const v = latest.current;
      if (e.ctrlKey || e.metaKey) {
        const rect = el.getBoundingClientRect(),
          x = e.clientX - rect.left,
          y = e.clientY - rect.top;
        const scale = Math.min(
          2,
          Math.max(0.18, v.scale * Math.exp(-e.deltaY * 0.008)),
        );
        setView({
          x: x - ((x - v.x) * scale) / v.scale,
          y: y - ((y - v.y) * scale) / v.scale,
          scale,
        });
      } else setView({ ...v, x: v.x - e.deltaX, y: v.y - e.deltaY });
    }
    el.addEventListener("wheel", wheel, { passive: false });
    return () => el.removeEventListener("wheel", wheel);
  }, []);
  const zoom = (factor: number) =>
    setView((v) => {
      const rect = root.current!.getBoundingClientRect(),
        x = rect.width / 2,
        y = rect.height / 2;
      const scale = Math.min(2, Math.max(0.18, v.scale * factor));
      return {
        x: x - ((x - v.x) * scale) / v.scale,
        y: y - ((y - v.y) * scale) / v.scale,
        scale,
      };
    });

  return { root, view, latest, setView, size, fit, zoom };
}
