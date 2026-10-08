import { expect, test } from "bun:test";
import { newProject } from "../model";
import {
  createProjectAutosave,
  SAVED_LABEL,
} from "../workspace/projectAutosave";
import {
  canvasViewport,
  forgetViewport,
  persistViewport,
  rememberViewport,
  rememberedViewport,
} from "./viewportMemory";

test("pan and zoom stay local; remembered views are isolated while layout edits still save", async () => {
  const original = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
  const data = new Map<string, string>();
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: {
      getItem: (key: string) => data.get(key) ?? null,
      setItem: (key: string, value: string) => data.set(key, value),
      removeItem: (key: string) => data.delete(key),
    },
  });
  const project = newProject("view memory");
  const other = newProject("other project");
  const before = structuredClone(project);
  let writes = 0;
  const save = createProjectAutosave(project, async (value) => {
    writes++;
    return value;
  });
  try {
    const view = { x: -600, y: -320, scale: 0.6 };
    rememberViewport(project.id, view);
    expect(canvasViewport(project)).toEqual(view);
    expect(canvasViewport(other)).toEqual(other.viewport);
    persistViewport(project.id);
    await save.flush();
    expect(writes).toBe(0);
    expect(save.getStatus()).toBe(SAVED_LABEL);
    expect(project).toEqual(before);

    const stored = new Map(data);
    forgetViewport(project.id);
    expect(data.size).toBe(0);
    for (const [key, value] of stored) data.set(key, value);
    expect(rememberedViewport(project.id)).toEqual(view);
    const legacy = { ...project, production: { viewport: other.viewport } };
    expect(canvasViewport(legacy)).toEqual(view);

    save.update({
      ...project,
      production: { positions: { card: { x: 30, y: 40 } } },
    });
    await save.flush();
    expect(writes).toBe(1);
    expect(canvasViewport(project)).toEqual(view);

    forgetViewport(project.id);
    for (const [key] of stored) data.set(key, '{"x":0,"y":0,"scale":0}');
    expect(canvasViewport(legacy)).toEqual(other.viewport);
  } finally {
    forgetViewport(project.id);
    forgetViewport(other.id);
    if (original) Object.defineProperty(globalThis, "localStorage", original);
    else Reflect.deleteProperty(globalThis, "localStorage");
  }
});
