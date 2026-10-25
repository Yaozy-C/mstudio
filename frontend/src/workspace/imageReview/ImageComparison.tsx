import { useRef, useState } from "react";
import { ArrowsHorizontal } from "@phosphor-icons/react";
import { t } from "../../i18n";
import "./image-comparison.css";

export function ImageComparison({
  original,
  natural,
  viewport,
  view,
  transform,
}: {
  original: { src: string; alt: string };
  natural: { width: number; height: number };
  viewport: { width: number; height: number };
  view: { scale: number; x: number; y: number };
  transform: string;
}) {
  const overlay = useRef<HTMLDivElement>(null);
  const pointer = useRef<number | null>(null);
  const [position, setPosition] = useState(0.5);
  const width = natural.width * view.scale;
  const height = natural.height * view.scale;
  const left = Math.max(0, (viewport.width - width) / 2 + view.x);
  const right = Math.min(viewport.width, (viewport.width + width) / 2 + view.x);
  const top = Math.max(0, (viewport.height - height) / 2 + view.y);
  const bottom = Math.min(
    viewport.height,
    (viewport.height + height) / 2 + view.y,
  );
  const split = left + (right - left) * position;
  const move = (clientX: number) => {
    const bounds = overlay.current?.getBoundingClientRect();
    if (bounds && right > left)
      setPosition(
        Math.max(
          0,
          Math.min(1, (clientX - bounds.left - left) / (right - left)),
        ),
      );
  };
  const finish = () => {
    pointer.current = null;
  };
  return (
    <div ref={overlay} className="image-comparison">
      <div
        className="image-comparison-original"
        style={{
          clipPath: `inset(0 ${Math.max(0, viewport.width - split)}px 0 0)`,
        }}
      >
        <img
          src={original.src}
          alt={original.alt}
          draggable={false}
          style={{ width: natural.width, height: natural.height, transform }}
        />
        <span
          className="image-comparison-label"
          style={{ left: left + 12, top: top + 12 }}
        >
          {t("原图")}
        </span>
      </div>
      <div
        className="image-comparison-result"
        style={{ clipPath: `inset(0 0 0 ${split}px)` }}
      >
        <span
          className="image-comparison-label"
          style={{ right: viewport.width - right + 48, top: top + 12 }}
        >
          {t("处理结果")}
        </span>
      </div>
      <button
        type="button"
        className="image-comparison-handle"
        role="slider"
        aria-label={t("原图与处理结果对比分割线")}
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(position * 100)}
        aria-valuetext={t("原图 {v0}% · 处理结果 {v1}%", {
          v0: Math.round(position * 100),
          v1: Math.round((1 - position) * 100),
        })}
        aria-orientation="horizontal"
        style={{ left: split, top, height: Math.max(0, bottom - top) }}
        onPointerDown={(event) => {
          if (event.button !== 0) return;
          event.preventDefault();
          event.stopPropagation();
          event.currentTarget.focus();
          event.currentTarget.setPointerCapture(event.pointerId);
          pointer.current = event.pointerId;
          move(event.clientX);
        }}
        onPointerMove={(event) => {
          if (pointer.current !== event.pointerId) return;
          event.stopPropagation();
          move(event.clientX);
        }}
        onPointerUp={finish}
        onPointerCancel={finish}
        onLostPointerCapture={finish}
        onDoubleClick={(event) => event.stopPropagation()}
        onKeyDown={(event) => {
          const direction =
            event.key === "ArrowLeft" ? -1 : event.key === "ArrowRight" ? 1 : 0;
          if (!direction && event.key !== "Home" && event.key !== "End") return;
          event.preventDefault();
          event.stopPropagation();
          setPosition((current) =>
            event.key === "Home"
              ? 0
              : event.key === "End"
                ? 1
                : Math.max(
                    0,
                    Math.min(
                      1,
                      current + direction * (event.shiftKey ? 0.1 : 0.01),
                    ),
                  ),
          );
        }}
      >
        <span className="image-comparison-grip" aria-hidden="true">
          <ArrowsHorizontal size={18} />
        </span>
      </button>
    </div>
  );
}
