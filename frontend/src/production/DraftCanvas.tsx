import { canvasNote } from "../workspace/canvasAsset";
import { draftItems } from "./items";
import { CanvasControls } from "./CanvasControls";
import { timelineAssetIds } from "./canvasIndex";
import { t, useLanguage } from "../i18n";
import { addStoryAsset } from "../workspace/storyAssetLibrary";
import { AdoptDraftDialog } from "./AdoptDraftDialog";
import "../workspace/story-assets.css";
import { useCallback, useMemo, useState } from "react";
import { useProductionViewport } from "./useProductionViewport";
import { visibleItems } from "./visibleItems";
import { CanvasCard } from "./CanvasCard";
import { StudioNodeEditor } from "../workspace/StudioNodeEditor";
import type { Asset, Project } from "../model";
import type { DraftItem, ChangeProject } from "./types";
import type { ProductionController } from "./useProduction";
import "./canvas.css";
export function DraftCanvas({
  project,
  change,
  canvas,
}: {
  project: Project;
  change: ChangeProject;
  canvas: ProductionController;
}) {
  useLanguage();
  const { items, selected, choose: setSelection, focus } = canvas;
  const assets = useMemo(
    () => new Map(project.assets.map((a) => [a.id, a])),
    [project.assets],
  );
  const timelineAssets = useMemo(
    () => timelineAssetIds(project.clips),
    [project.clips],
  );
  const { root, view, latest, setView, size, fit, zoom } =
    useProductionViewport(project, items, focus);
  const visible = visibleItems(items, view, size);
  const collect = useCallback(
    (asset: Asset) => change((p) => addStoryAsset(p, asset.id)),
    [change],
  );
  const [adopting, setAdopting] = useState<Asset | null>(null);
  const [preview, setPreview] = useState<DraftItem | null>(null);
  const [box, setBox] = useState<{
    x: number;
    y: number;
    width: number;
    height: number;
  } | null>(null);
  return (
    <div className="production-workspace">
      <header className="draft-canvas-heading">
        <div>
          <h1>{t("草稿画布")}</h1>
          <p>{t("自由摆放想法与参考，试生成后再决定是否用于正式制作。")}</p>
        </div>
        <button
          onClick={() => {
            const node = canvasNote(project, "");
            const next = { ...project, nodes: [...project.nodes, node] };
            change((p) => ({ ...p, nodes: [...p.nodes, node] }));
            setPreview(draftItems(next).find((i) => i.nodeId === node.id)!);
          }}
        >
          {t("新建草稿笔记")}
        </button>
      </header>
      <div
        ref={root}
        className="creation-canvas"
        aria-label={t("草稿画布")}
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
          data-detail={view.scale < 0.75 ? "summary" : "full"}
          style={{
            transform: `t(${view.x}px,${view.y}px) scale(${view.scale})`,
          }}
        >
          {visible.map((item) => (
            <CanvasCard
              onAdd={setAdopting}
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
      {adopting && (
        <AdoptDraftDialog
          asset={adopting}
          project={project}
          change={change}
          close={() => setAdopting(null)}
        />
      )}
      {preview && (
        <StudioNodeEditor
          add={setAdopting}
          reference={(id) => canvas.reference([`node:${id}`])}
          key={preview.key}
          node={project.nodes.find((n) => n.id === preview.nodeId)!}
          project={project}
          change={change}
          close={() => setPreview(null)}
        />
      )}
    </div>
  );
}
