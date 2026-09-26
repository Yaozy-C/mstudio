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
  project,
  clock,
  zoom,
  viewport,
  onChange,
  onCaption,
}: {
  project: Project;
  clock: PlaybackClock;
  zoom: number;
  viewport: { left: number; width: number };
  onChange: (f: (p: Project) => Project) => void;
  onCaption?: () => void;
}) {
  const location = useRef(0);
  const add = () => {
    const start = Math.max(
      0,
      Math.round(location.current * project.fps) / project.fps,
    );
    clock.pause();
    requestAnimationFrame(() => clock.seek(start));
    onChange((p) => ({
      ...p,
      captions: [
        ...(p.captions ?? []),
        { id: uid(), start, end: start + 3, text: "新字幕" },
      ],
    }));
    onCaption?.();
  };
  return (
    <ObjectMenu actions={[{ label: "在此添加字幕", run: add }]}>
      <div
        className="caption-track"
        aria-label="字幕轨道：双击空白添加字幕"
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
        {!project.captions?.length && (
          <span className="track-empty" style={{ pointerEvents: "none" }}>
            双击添加字幕 · 右键更多操作
          </span>
        )}
        {(project.captions ?? [])
          .filter(
            (c) =>
              c.end * zoom >= viewport.left &&
              c.start * zoom <= viewport.left + viewport.width,
          )
          .map((c) => (
            <CaptionChip
              key={c.id}
              caption={c}
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
  caption: c,
  zoom,
  fps,
  clock,
  open,
  change,
}: {
  caption: Caption;
  zoom: number;
  fps: number;
  clock: PlaybackClock;
  open?: () => void;
  change: (f: (p: Project) => Project) => void;
}) {
  const drag = useRef<{ x: number; side: string; moved: boolean } | null>(null);
  const patch = (next: Caption) =>
    change((p) => ({
      ...p,
      captions: p.captions.map((v) => (v.id === c.id ? next : v)),
    }));
  return (
    <ObjectMenu
      actions={[
        {
          label: "编辑字幕",
          run: () => {
            clock.pause();
            clock.seek(c.start);
            open?.();
          },
        },
        {
          label: "删除字幕",
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
        className="caption-chip"
        style={{
          left: c.start * zoom,
          width: Math.max(8, (c.end - c.start) * zoom),
          touchAction: "none",
          cursor: "grab",
        }}
        title="拖动移动字幕 · 两端调整时长 · 双击编辑"
        onDoubleClick={(e) => {
          e.stopPropagation();
          open?.();
        }}
        onPointerDown={(e) => {
          e.stopPropagation();
          if (e.button !== 0) return;
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
          const next = moveCaption(c, (e.clientX - d.x) / zoom, fps, d.side);
          e.currentTarget.style.left = `${next.start * zoom}px`;
          e.currentTarget.style.width = `${Math.max(8, (next.end - next.start) * zoom)}px`;
        }}
        onPointerUp={(e) => {
          const d = drag.current;
          if (!d) return;
          drag.current = null;
          e.currentTarget.releasePointerCapture(e.pointerId);
          if (d.moved)
            patch(moveCaption(c, (e.clientX - d.x) / zoom, fps, d.side));
          else clock.seek(c.start);
        }}
        onPointerCancel={(e) => {
          drag.current = null;
          e.currentTarget.style.left = `${c.start * zoom}px`;
          e.currentTarget.style.width = `${(c.end - c.start) * zoom}px`;
        }}
        onKeyDown={(e) => {
          if (e.key === "ArrowLeft" || e.key === "ArrowRight") {
            e.preventDefault();
            patch(moveCaption(c, (e.key === "ArrowLeft" ? -1 : 1) / fps, fps));
          }
        }}
      >
        {c.text}
      </button>
    </ObjectMenu>
  );
}
