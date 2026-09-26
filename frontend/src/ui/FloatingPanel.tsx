import { useEffect, useState, type ReactNode } from "react";
import { DotsSix, Minus, X, Graph, SquaresFour } from "@phosphor-icons/react";
import type { Bounds, ResizeAxis } from "./panelGeometry";
import { usePanelBounds } from "./usePanelBounds";
export function FloatingPanel({
  id,
  title,
  initial,
  children,
  onClose,
  className = "",
  visible = true,
  reveal = 0,
  bottomInset = 0,
}: {
  id: string;
  title: string;
  initial: Bounds;
  children: ReactNode;
  onClose: () => void;
  className?: string;
  visible?: boolean;
  reveal?: number;
  bottomInset?: number;
}) {
  const [collapsedAt, setCollapsedAt] = useState<number | null>(null);
  const collapsed = collapsedAt === reveal;
  const [front, setFront] = useState(false);
  const geometry = usePanelBounds(id, initial, collapsed, bottomInset);
  useEffect(() => {
    if (reveal) setFront(true);
  }, [reveal]);
  return (
    <section
      ref={geometry.ref}
      className={`floating-panel ${className} ${collapsed ? "collapsed" : ""}`}
      aria-label={title}
      style={{
        ...geometry.style,
        display: visible ? undefined : "none",
        zIndex: id === "agent" ? (front ? 32 : 29) : front ? 24 : 20,
      }}
      onPointerDown={() => setFront(true)}
      onBlur={(e) => {
        if (!e.currentTarget.contains(e.relatedTarget)) setFront(false);
      }}
    >
      <header className="floating-handle" {...geometry.handlers("move")}>
        <DotsSix />
        {collapsed ? (
          <button
            className="panel-restore"
            title={`展开${title}`}
            aria-expanded={false}
            onClick={() => setCollapsedAt(null)}
          >
            {id === "agent" ? <Graph /> : <SquaresFour />}
            <span>{id === "agent" ? "Agent" : title}</span>
          </button>
        ) : (
          <>
            <strong>
              {id === "agent" && <Graph size={18} />}
              {title}
            </strong>
            <button
              title={`折叠${title}`}
              className="icon-button"
              aria-expanded={true}
              onClick={() => setCollapsedAt(reveal)}
            >
              <Minus />
            </button>
            <button
              title={`关闭${title}`}
              className="icon-button"
              onClick={onClose}
            >
              <X />
            </button>
          </>
        )}
      </header>
      <div className="floating-content" hidden={collapsed}>
        {children}
      </div>
      {!collapsed &&
        (["height", "width", "both"] as ResizeAxis[]).map((axis) => (
          <div
            key={axis}
            className={`panel-resizer panel-resizer-${axis}`}
            role="separator"
            tabIndex={0}
            aria-label={`调整${title}${axis === "height" ? "高度" : axis === "width" ? "宽度" : "尺寸"}`}
            aria-orientation={axis === "width" ? "vertical" : "horizontal"}
            aria-valuenow={Math.round(
              axis === "width"
                ? geometry.expanded.width
                : geometry.expanded.height,
            )}
            aria-valuemin={axis === "width" ? 260 : 180}
            aria-valuemax={Math.round(
              axis === "width"
                ? window.innerWidth - geometry.expanded.x - 12
                : window.innerHeight - geometry.expanded.y - 12,
            )}
            title={
              axis === "height"
                ? "拖动底边调整高度 · ↑↓ 微调"
                : axis === "width"
                  ? "拖动右边调整宽度 · ←→ 微调"
                  : "拖动调整面板尺寸"
            }
            {...geometry.handlers(axis)}
            onKeyDown={(e) => {
              if (
                !["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(
                  e.key,
                )
              )
                return;
              e.preventDefault();
              e.stopPropagation();
              const step = e.shiftKey ? 64 : 16;
              geometry.resizeBy(
                e.key === "ArrowRight"
                  ? step
                  : e.key === "ArrowLeft"
                    ? -step
                    : 0,
                e.key === "ArrowDown" ? step : e.key === "ArrowUp" ? -step : 0,
                axis,
              );
            }}
          />
        ))}
    </section>
  );
}
