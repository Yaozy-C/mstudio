import type { ImageComment, Point } from "./annotations";

export type AnnotationControls = {
  enabled: boolean;
  items: ImageComment[];
  selected: string | null;
  add: (region: Omit<ImageComment, "id" | "text">) => void;
  select: (id: string) => void;
};
export function AnnotationOverlay({
  annotations,
  pending,
  width,
  height,
  transform,
  scale,
}: {
  annotations: AnnotationControls;
  pending: { start: Point; end: Point } | null;
  width: number;
  height: number;
  transform: string;
  scale: number;
}) {
  return (
    <svg
      className="image-annotation-overlay"
      viewBox="0 0 100 100"
      preserveAspectRatio="none"
      style={{ width, height, transform }}
      aria-hidden="true"
    >
      {annotations.items.map((c, i) => (
        <g
          key={c.id}
          className={annotations.selected === c.id ? "selected" : ""}
          onPointerDown={(event) => {
            event.stopPropagation();
            event.preventDefault();
            annotations.select(c.id);
          }}
        >
          <rect
            x={c.x * 100}
            y={c.y * 100}
            width={c.width * 100}
            height={c.height * 100}
          />
          <svg
            x={c.x * 100}
            y={c.y * 100}
            width={(28 / scale / width) * 100}
            height={(28 / scale / height) * 100}
            viewBox="0 0 32 32"
            preserveAspectRatio="none"
            overflow="visible"
          >
            <circle cx="0" cy="0" r="12" />
            <text x="0" y="0" dominantBaseline="central" textAnchor="middle">
              {i + 1}
            </text>
          </svg>
        </g>
      ))}
      {pending && (
        <rect
          className="pending"
          x={Math.min(pending.start.x, pending.end.x) * 100}
          y={Math.min(pending.start.y, pending.end.y) * 100}
          width={Math.abs(pending.end.x - pending.start.x) * 100}
          height={Math.abs(pending.end.y - pending.start.y) * 100}
        />
      )}
    </svg>
  );
}
