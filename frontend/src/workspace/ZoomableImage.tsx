import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { ZoomControls } from "../ui/ZoomControls";
import { t, useLanguage } from "../i18n";

import {
  AnnotationOverlay,
  type AnnotationControls,
} from "./imageReview/AnnotationOverlay";
import {
  imagePoint,
  commentRegion,
  type Point,
} from "./imageReview/annotations";

type View = { scale: number; x: number; y: number };
export function ZoomableImage({
  src,
  alt,
  annotations,
}: {
  src: string;
  alt: string;
  annotations?: AnnotationControls;
}) {
  useLanguage();
  const viewport = useRef<HTMLDivElement>(null);
  const drag = useRef<{ id: number; x: number; y: number } | null>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  const [natural, setNatural] = useState({ width: 0, height: 0 });
  const [view, setView] = useState<View>({ scale: 1, x: 0, y: 0 });
  const [dragging, setDragging] = useState(false);
  const [pending, setPending] = useState<{ start: Point; end: Point } | null>(
    null,
  );
  const region = useRef<{ start: Point; end: Point } | null>(null);
  const pointAt = (clientX: number, clientY: number) => {
    const bounds = viewport.current?.getBoundingClientRect();
    return bounds
      ? imagePoint(
          { x: clientX - bounds.left, y: clientY - bounds.top },
          size,
          natural,
          view,
        )
      : null;
  };
  const transform = `translate(-50%, -50%) translate(${view.x}px, ${view.y}px) scale(${view.scale})`;
  const fit =
    natural.width && size.width
      ? Math.min(size.width / natural.width, size.height / natural.height)
      : 1;
  const minimum = Math.min(fit, 0.1);
  const maximum = Math.max(fit, 8);
  const constrain = (next: View): View => {
    const x = Math.max(0, (natural.width * next.scale - size.width) / 2);
    const y = Math.max(0, (natural.height * next.scale - size.height) / 2);
    return {
      ...next,
      x: Math.max(-x, Math.min(x, next.x)),
      y: Math.max(-y, Math.min(y, next.y)),
    };
  };
  const zoom = (scale: number, anchor = { x: 0, y: 0 }) => {
    setView((previous) => {
      const next = Math.max(minimum, Math.min(maximum, scale));
      const ratio = next / previous.scale;
      return constrain({
        scale: next,
        x: anchor.x + (previous.x - anchor.x) * ratio,
        y: anchor.y + (previous.y - anchor.y) * ratio,
      });
    });
  };
  useLayoutEffect(() => {
    const element = viewport.current;
    if (!element) return;
    const observer = new ResizeObserver(() =>
      setSize({ width: element.clientWidth, height: element.clientHeight }),
    );
    observer.observe(element);
    return () => observer.disconnect();
  }, []);
  useLayoutEffect(() => {
    setView({ scale: fit, x: 0, y: 0 });
  }, [fit, src]);
  useEffect(() => {
    const element = viewport.current;
    if (!element) return;
    const wheel = (event: WheelEvent) => {
      event.preventDefault();
      event.stopPropagation();
      if (!event.ctrlKey && !event.metaKey) {
        const unit =
          event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? size.height : 1;
        setView((current) =>
          constrain({
            ...current,
            x: current.x - event.deltaX * unit,
            y: current.y - event.deltaY * unit,
          }),
        );
        return;
      }
      const bounds = element.getBoundingClientRect();
      const delta =
        event.deltaY *
        (event.deltaMode === 1
          ? 16
          : event.deltaMode === 2
            ? bounds.height
            : 1);
      zoom(
        view.scale * Math.exp(-Math.max(-100, Math.min(100, delta)) * 0.005),
        {
          x: event.clientX - bounds.left - bounds.width / 2,
          y: event.clientY - bounds.top - bounds.height / 2,
        },
      );
    };
    element.addEventListener("wheel", wheel, { passive: false });
    return () => element.removeEventListener("wheel", wheel);
  }, [view.scale, fit, natural.width, natural.height, size.width, size.height]);
  const reset = () => setView({ scale: fit, x: 0, y: 0 });
  const finishDrag = () => {
    drag.current = null;
    setDragging(false);
    region.current = null;
    setPending(null);
  };
  return (
    <div className="zoomable-image">
      <div
        ref={viewport}
        className="image-zoom-viewport"
        data-dragging={dragging}
        data-annotating={!!annotations?.enabled}
        tabIndex={0}
        role="region"
        aria-label={t("图片细节预览")}
        onKeyDown={(event) => {
          if (["+", "=", "-", "0", "1"].includes(event.key)) {
            event.preventDefault();
            if (event.key === "0") reset();
            else
              zoom(
                event.key === "1"
                  ? 1
                  : view.scale * (event.key === "-" ? 1 / 1.25 : 1.25),
              );
          }
        }}
        onDoubleClick={() =>
          annotations?.enabled
            ? undefined
            : Math.abs(view.scale - fit) < 0.001
              ? zoom(Math.max(1, fit * 2))
              : reset()
        }
        onPointerDown={(event) => {
          if (event.button !== 0) return;
          event.preventDefault();
          event.currentTarget.focus();
          event.currentTarget.setPointerCapture(event.pointerId);
          if (annotations?.enabled) {
            const point = pointAt(event.clientX, event.clientY);
            if (point) {
              region.current = { start: point, end: point };
              setPending(region.current);
            }
            return;
          }
          drag.current = {
            id: event.pointerId,
            x: event.clientX,
            y: event.clientY,
          };
          setDragging(true);
        }}
        onPointerMove={(event) => {
          if (region.current) {
            const point = pointAt(event.clientX, event.clientY);
            if (point) {
              region.current.end = point;
              setPending({ ...region.current });
            }
            return;
          }
          const previous = drag.current;
          if (!previous || previous.id !== event.pointerId) return;
          const dx = event.clientX - previous.x,
            dy = event.clientY - previous.y;
          drag.current = {
            id: event.pointerId,
            x: event.clientX,
            y: event.clientY,
          };
          setView((current) =>
            constrain({ ...current, x: current.x + dx, y: current.y + dy }),
          );
        }}
        onPointerUp={() => {
          if (region.current)
            annotations?.add(
              commentRegion(region.current.start, region.current.end),
            );
          finishDrag();
        }}
        onPointerCancel={finishDrag}
        onLostPointerCapture={finishDrag}
      >
        <img
          src={src}
          alt={alt}
          draggable={false}
          onLoad={(event) =>
            setNatural({
              width: event.currentTarget.naturalWidth,
              height: event.currentTarget.naturalHeight,
            })
          }
          style={{
            width: natural.width || undefined,
            height: natural.height || undefined,
            visibility: natural.width ? "visible" : "hidden",
            transform,
          }}
        />
        {annotations && natural.width > 0 && (
          <AnnotationOverlay
            annotations={annotations}
            pending={pending}
            width={natural.width}
            height={natural.height}
            transform={transform}
            scale={view.scale}
          />
        )}
      </div>
      <p className="image-zoom-hint">
        {annotations?.enabled
          ? t("拖动框选修改区域 · ⌘/Ctrl + 滚轮缩放")
          : t("拖动平移 · ⌘/Ctrl + 滚轮缩放 · 双击切换")}
      </p>
      <ZoomControls
        image
        scale={view.scale}
        zoom={(factor) => zoom(view.scale * factor)}
        fit={reset}
        minimum={minimum}
        maximum={maximum}
      />
    </div>
  );
}
