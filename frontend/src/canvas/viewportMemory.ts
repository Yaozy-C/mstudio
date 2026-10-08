import type { Project } from "../model";

type Viewport = Project["viewport"];
const views = new Map<string, Viewport>();
const key = (id: string) => `mstudio-canvas-view:${id}`;

export function rememberedViewport(id: string): Viewport | undefined {
  const current = views.get(id);
  if (current) return current;
  try {
    const saved = JSON.parse(localStorage.getItem(key(id)) || "null");
    if (
      saved &&
      Number.isFinite(saved.x) &&
      Number.isFinite(saved.y) &&
      Number.isFinite(saved.scale) &&
      saved.scale > 0
    ) {
      const view = { x: saved.x, y: saved.y, scale: saved.scale };
      views.set(id, view);
      return view;
    }
  } catch {
    /* Missing or unavailable preferences fall back to the project view. */
  }
}

export function canvasViewport(project: Project): Viewport {
  return (
    rememberedViewport(project.id) ??
    project.production?.viewport ??
    project.viewport
  );
}

export function rememberViewport(id: string, view: Viewport) {
  views.set(id, view);
}

export function persistViewport(id: string) {
  const view = views.get(id);
  if (!view) return;
  try {
    localStorage.setItem(key(id), JSON.stringify(view));
  } catch {
    /* Keep the view for this session if local preferences cannot be saved. */
  }
}

export function forgetViewport(id: string) {
  views.delete(id);
  try {
    localStorage.removeItem(key(id));
  } catch {
    /* Project deletion does not depend on optional UI preferences. */
  }
}
