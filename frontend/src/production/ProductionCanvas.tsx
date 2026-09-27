import { CanvasControls } from "./CanvasControls";
import { t, useLanguage } from "../i18n";
import { collectAsset } from "../workspace/assetLibrary";
import { productionShots } from "./items";
import { useCallback, useEffect, useRef, useState } from "react";
import { fitView } from "../canvas/fit";
import { useViewport } from "../canvas/useViewport";
import { CanvasCard } from "./CanvasCard";
import { CanvasPreview } from "./CanvasPreview";
import type { Asset, Project } from "../model";
import type { ProductionItem, ChangeProject } from "./types";
import type { ProductionController } from "./useProduction";
import "./canvas.css";
export function ProductionCanvas({
  project,
  change,
  canvas,
  onAdd,
}: {
  project: Project;
  change: ChangeProject;
  canvas: ProductionController;
  onAdd: (asset: Asset) => void;
}) {
  useLanguage();
  const { items, selected, choose: setSelection, focus } = canvas;
  const shots = productionShots(project);
  const root = useRef<HTMLDivElement>(null);
  const storeView = useCallback(
    (fn: (p: Project) => Project) =>
      change(
        (p) => ({
          ...p,
          production: { ...p.production, viewport: fn(p).viewport },
        }),
        false,
      ),
    [change],
  );
  const { view, latest, update } = useViewport(
    project.production?.viewport ?? project.viewport,
    storeView,
  );
  const setView = (
    value:
      Project["viewport"] | ((v: Project["viewport"]) => Project["viewport"]),
  ) => update(typeof value === "function" ? value(latest.current) : value);
  const [preview, setPreview] = useState<ProductionItem | null>(null);
  const [box, setBox] = useState<{
    x: number;
    y: number;
    width: number;
    height: number;
  } | null>(null);
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
    if (focus.tick || !project.production?.viewport) fit();
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
  return (
    <div className="production-workspace">
      <div
        ref={root}
        className="creation-canvas"
        aria-label={t("镜头制作画布")}
        style={{
          backgroundPosition: `${view.x}px ${view.y}px`,
          backgroundSize: `${24 * view.scale}px ${24 * view.scale}px`,
        }}
        onPointerDown={(e) => {
          // Portaled menus still bubble through React's canvas tree. Only
          // capture pointers that physically started inside the canvas.
          if (!e.currentTarget.contains(e.target as Node)) return;
          if (
            (e.target as HTMLElement).closest("article,button,.canvas-controls")
          )
            return;
          if (e.button !== 0) return;
          const el = e.currentTarget,
            rect = el.getBoundingClientRect();
          const sx = e.clientX,
            sy = e.clientY,
            start = latest.current;
          el.setPointerCapture(e.pointerId);
          const drag = (p: PointerEvent) => {
            if (e.shiftKey)
              setBox({
                x: Math.min(sx, p.clientX) - rect.left,
                y: Math.min(sy, p.clientY) - rect.top,
                width: Math.abs(p.clientX - sx),
                height: Math.abs(p.clientY - sy),
              });
            else
              setView({
                ...start,
                x: start.x + p.clientX - sx,
                y: start.y + p.clientY - sy,
              });
          };
          const end = (p: PointerEvent) => {
            if (e.shiftKey) {
              const left =
                (Math.min(sx, p.clientX) - rect.left - start.x) / start.scale;
              const top =
                (Math.min(sy, p.clientY) - rect.top - start.y) / start.scale;
              const right = left + Math.abs(p.clientX - sx) / start.scale,
                bottom = top + Math.abs(p.clientY - sy) / start.scale;
              setSelection(
                items
                  .filter(
                    (n) =>
                      n.x < right &&
                      n.x + n.width > left &&
                      n.y < bottom &&
                      n.y + n.height > top,
                  )
                  .map((n) => n.key),
              );
            }
            if (
              !e.shiftKey &&
              Math.abs(p.clientX - sx) + Math.abs(p.clientY - sy) < 3
            )
              setSelection([]);
            setBox(null);
            el.removeEventListener("pointermove", drag);
            el.removeEventListener("pointercancel", end);
          };
          el.addEventListener("pointermove", drag);
          el.addEventListener("pointerup", end, { once: true });
          el.addEventListener("pointercancel", end, { once: true });
        }}
      >
        <div
          className="canvas-world"
          data-scale={view.scale}
          style={{
            transform: `translate(${view.x}px,${view.y}px) scale(${view.scale})`,
          }}
        >
          {shots.map((n, i) => (
            <div
              className="shot-area"
              key={n.id}
              style={{
                left: i * 1600,
                height: Math.max(
                  680,
                  ...items
                    .filter((v) => v.ownerId === n.id)
                    .map((v) => v.y + v.height + 30),
                ),
              }}
            >
              <span>SHOT {String(i + 1).padStart(2, "0")}</span>
              <h2>{n.title}</h2>
            </div>
          ))}
          {items.map((item) => (
            <CanvasCard
              onAdd={onAdd}
              key={item.key}
              item={item}
              project={project}
              canvas={canvas}
              preview={setPreview}
              collect={() => {
                const asset = project.assets.find((a) => a.id === item.assetId);
                if (asset) change((p) => collectAsset(p, asset));
              }}
            />
          ))}
        </div>
        {box && (
          <div
            className="selection-box"
            style={{
              left: box.x,
              top: box.y,
              width: box.width,
              height: box.height,
            }}
          />
        )}
        <CanvasControls
          scale={view.scale}
          zoom={zoom}
          fit={() => fit(true)}
          count={selected.length}
          reference={() => canvas.reference(selected)}
          empty={!items.length}
        />
      </div>
      {preview && (
        <CanvasPreview
          onAdd={onAdd}
          key={preview.key}
          item={preview}
          project={project}
          change={change}
          close={() => setPreview(null)}
        />
      )}
    </div>
  );
}
