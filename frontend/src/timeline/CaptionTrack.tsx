import { captionTrackId } from "./captionTracks";
import { t, useLanguage } from "../i18n";
import { Check, Subtitles } from "@phosphor-icons/react";
import { useRef } from "react";
import { uid, type Caption, type Project } from "../model";
import type { PlaybackClock } from "./clock";
import { ObjectMenu } from "../ui/ObjectMenu";
export function moveCaption(
  c: Caption,
  delta: number,
  fps: number,
  side = "move",
) {
  const snap = (v: number) => Math.round(v * fps) / fps;
  if (side === "left")
    return {
      ...c,
      start: Math.max(0, Math.min(c.end - 1 / fps, snap(c.start + delta))),
    };
  if (side === "right")
    return { ...c, end: Math.max(c.start + 1 / fps, snap(c.end + delta)) };
  const start = Math.max(0, snap(c.start + delta));
  return { ...c, start, end: start + c.end - c.start };
}
export function CaptionTrack({
  trackId,
  selected,
  onSelect,
  project,
  clock,
  zoom,
  viewport,
  onChange,
  onCaption,
}: {
  trackId: string;
  selected?: string | null;
  onSelect?: (id: string | null) => void;
  project: Project;
  clock: PlaybackClock;
  zoom: number;
  viewport: { left: number; width: number };
  onChange: (f: (p: Project) => Project) => void;
  onCaption?: () => void;
}) {
  useLanguage();
  const location = useRef(0);
  const add = () => {
    const start = Math.max(
      0,
      Math.round(location.current * project.fps) / project.fps,
    );
    clock.pause();
    requestAnimationFrame(() => clock.seek(start));
    const id = uid();
    onChange((p) => ({
      ...p,
      captions: [
        ...(p.captions ?? []),
        { id, trackId, start, end: start + 3, text: t("新字幕") },
      ],
    }));
    onSelect?.(id);
    onCaption?.();
  };
  return (
    <ObjectMenu actions={[{ label: t("在此添加字幕"), run: add }]}>
      <div
        className="caption-track"
        aria-label={t("字幕轨道：双击空白添加字幕")}
        onPointerDown={(e) => {
          location.current =
            (e.clientX - e.currentTarget.getBoundingClientRect().left) / zoom;
        }}
        onContextMenu={(e) => {
          location.current =
            (e.clientX - e.currentTarget.getBoundingClientRect().left) / zoom;
        }}
        onDoubleClick={(e) => {
          if (e.target === e.currentTarget) add();
        }}
      >
        {!project.captions.some(
          (c) => captionTrackId(project, c) === trackId,
        ) && (
          <span className="track-empty" style={{ pointerEvents: "none" }}>
            {t("双击添加字幕 · 右键更多操作")}
          </span>
        )}
        {(project.captions ?? [])
          .filter(
            (c) =>
              captionTrackId(project, c) === trackId &&
              c.end * zoom >= viewport.left &&
              c.start * zoom <= viewport.left + viewport.width,
          )
          .map((c) => (
            <CaptionChip
              key={c.id}
              caption={c}
              selected={selected === c.id}
              select={() => onSelect?.(c.id)}
              deselect={() => onSelect?.(null)}
              zoom={zoom}
              fps={project.fps}
              clock={clock}
              open={onCaption}
              change={onChange}
            />
          ))}
      </div>
    </ObjectMenu>
  );
}
function CaptionChip({
  selected,
  select,
  deselect,
  caption: c,
  zoom,
  fps,
  clock,
  open,
  change,
}: {
  selected: boolean;
  select: () => void;
  deselect: () => void;
  caption: Caption;
  zoom: number;
  fps: number;
  clock: PlaybackClock;
  open?: () => void;
  change: (f: (p: Project) => Project) => void;
}) {
  useLanguage();
  const drag = useRef<{ x: number; side: string; moved: boolean } | null>(null);
  const patch = (next: Caption) =>
    change((p) => ({
      ...p,
      captions: p.captions.map((v) =>
        v.id === c.id
          ? {
              ...next,
              words:
                next.end - next.start === c.end - c.start
                  ? next.words
                  : undefined,
              assetId: undefined,
            }
          : v,
      ),
    }));
  return (
    <ObjectMenu
      actions={[
        {
          label: t("编辑字幕"),
          run: () => {
            select();
            clock.pause();
            clock.seek(c.start);
            open?.();
          },
        },
        {
          label: t("删除字幕"),
          danger: true,
          run: () =>
            change((p) => ({
              ...p,
              captions: p.captions.filter((v) => v.id !== c.id),
            })),
        },
      ]}
    >
      <button
        className={`caption-chip ${selected ? "selected" : ""}`}
        aria-pressed={selected}
        aria-label={t("{v0} {v1}秒", { v0: c.text, v1: c.start.toFixed(2) })}
        onContextMenu={select}
        onClick={(e) => {
          if (e.detail === 0) {
            select();
            clock.pause();
            clock.seek(c.start);
          }
        }}
        style={{
          left: c.start * zoom,
          width: Math.max(8, (c.end - c.start) * zoom),
          touchAction: "none",
        }}
        title={t("拖动移动字幕 · 两端调整时长 · 双击编辑")}
        onDoubleClick={(e) => {
          e.stopPropagation();
          open?.();
        }}
        onPointerDown={(e) => {
          e.stopPropagation();
          if (e.button !== 0) return;
          select();
          clock.pause();
          const r = e.currentTarget.getBoundingClientRect();
          drag.current = {
            x: e.clientX,
            side:
              e.clientX - r.left < 7
                ? "left"
                : r.right - e.clientX < 7
                  ? "right"
                  : "move",
            moved: false,
          };
          e.currentTarget.setPointerCapture(e.pointerId);
        }}
        onPointerMove={(e) => {
          const d = drag.current;
          if (!d) return;
          if (Math.abs(e.clientX - d.x) > 3) d.moved = true;
          if (!d.moved) return;
          e.currentTarget.dataset.interaction =
            d.side === "move" ? "dragging" : "trimming";
          const next = moveCaption(c, (e.clientX - d.x) / zoom, fps, d.side);
          e.currentTarget.style.left = `${next.start * zoom}px`;
          e.currentTarget.style.width = `${Math.max(8, (next.end - next.start) * zoom)}px`;
        }}
        onPointerUp={(e) => {
          const d = drag.current;
          if (!d) return;
          drag.current = null;
          delete e.currentTarget.dataset.interaction;
          e.currentTarget.releasePointerCapture(e.pointerId);
          if (d.moved)
            patch(moveCaption(c, (e.clientX - d.x) / zoom, fps, d.side));
          else clock.seek(c.start);
        }}
        onPointerCancel={(e) => {
          drag.current = null;
          delete e.currentTarget.dataset.interaction;
          e.currentTarget.style.left = `${c.start * zoom}px`;
          e.currentTarget.style.width = `${(c.end - c.start) * zoom}px`;
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            select();
            open?.();
          }
          if (e.key === " " && !selected) {
            e.preventDefault();
            select();
            clock.pause();
            clock.seek(c.start);
          }
          if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "b")
            e.preventDefault();
          if (e.key === "Delete" || e.key === "Backspace") {
            e.preventDefault();
            change((p) => ({
              ...p,
              captions: p.captions.filter((v) => v.id !== c.id),
            }));
            deselect();
          }
          if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
            e.preventDefault();
            select();
            patch(moveCaption(c, (e.key === "ArrowLeft" ? -1 : 1) / fps, fps));
          }
        }}
      >
        {selected && (
          <Check
            className="clip-selection-mark"
            weight="bold"
            aria-hidden="true"
          />
        )}
        <Subtitles className="caption-type-icon" aria-hidden="true" />
        <span className="caption-chip-content">
          <span>{c.text}</span>
          <small>
            {(c.end - c.start).toFixed(2)} {t("秒")}
          </small>
        </span>
        <span className="caption-edge caption-edge-left" aria-hidden="true" />
        <span className="caption-edge caption-edge-right" aria-hidden="true" />
      </button>
    </ObjectMenu>
  );
}
