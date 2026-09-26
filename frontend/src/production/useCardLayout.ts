import { useCallback, useState } from "react";
import type { Project } from "../model";
import type { ProductionItem } from "./types";
export function useCardLayout(raw: ProductionItem[], project: Project) {
  const [heights, setHeights] = useState<Record<string, number>>({});
  const measure = useCallback((key: string, height: number) => {
    setHeights((old) =>
      old[key] === height ? old : { ...old, [key]: height },
    );
  }, []);
  const arranged: ProductionItem[] = [];
  const items = raw.map((item) => {
    if (item.kind === "script")
      return { ...item, height: heights[item.key] ?? item.height };
    if (
      !item.ownerId ||
      !item.key.startsWith("reference:") ||
      project.production?.positions?.[item.key]
    )
      return item;
    const placed = [
      ...arranged,
      ...raw.filter(
        (other) =>
          other.key !== item.key &&
          (other.kind === "script" ||
            !other.key.startsWith("reference:") ||
            project.production?.positions?.[other.key]),
      ),
    ];
    const x = Math.max(
      item.x,
      ...raw
        .filter(
          (other) =>
            other.ownerId === item.ownerId &&
            !other.key.startsWith("reference:"),
        )
        .map((other) => other.x + other.width + 32),
    );
    let y = item.y;
    while (
      placed.some(
        (other) =>
          other.x < x + item.width + 20 &&
          other.x + other.width + 20 > x &&
          other.y < y + item.height + 20 &&
          other.y + (heights[other.key] ?? other.height) + 20 > y,
      )
    )
      y += 395;
    const next = { ...item, x, y };
    arranged.push(next);
    return next;
  });
  return { items, measure };
}
