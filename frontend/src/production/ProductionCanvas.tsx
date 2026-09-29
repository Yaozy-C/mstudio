import { CanvasControls } from "./CanvasControls";
import { shotAreas, timelineAssetIds } from "./canvasIndex";
import { t, useLanguage } from "../i18n";
import { collectAsset } from "../workspace/assetLibrary";
import { productionShots } from "./items";
import { useCallback, useMemo, useState } from "react";
import { useProductionViewport } from "./useProductionViewport";
import { visibleItems } from "./visibleItems";
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
  const shots = useMemo(() => productionShots(project), [project.nodes]);
  const assets = useMemo(
    () => new Map(project.assets.map((a) => [a.id, a])),
    [project.assets],
  );
  const timelineAssets = useMemo(
    () => timelineAssetIds(project.clips),
    [project.clips],
  );
  const { root, view, latest, setView, size, fit, zoom } =
    useProductionViewport(project, change, items, focus, shots);
  const visible = visibleItems(items, view, size);
  const areas = useMemo(() => shotAreas(shots, items), [shots, items]);
  const collect = useCallback(
    (asset: Asset) => change((p) => collectAsset(p, asset)),
    [change],
  );
  const [preview, setPreview] = useState<ProductionItem | null>(null);
  const [box, setBox] = useState<{
    x: number;
    y: number;
    width: number;
    height: number;
  } | null>(null);
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
          {visibleItems(areas, view, size).map((n) => (
            <div
              className="shot-area"
              key={n.id}
              style={{ left: n.x, height: n.height }}
            >
              <span>SHOT {String(n.index + 1).padStart(2, "0")}</span>
              <h2>{n.title}</h2>
            </div>
          ))}
          {visible.map((item) => (
            <CanvasCard
              onAdd={onAdd}
              key={item.key}
              item={item}
              compact={view.scale < 0.35}
              asset={assets.get(item.assetId ?? "")}
              inTimeline={timelineAssets.has(item.assetId ?? "")}
              project={project}
              canvas={canvas}
              preview={setPreview}
              collect={collect}
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
