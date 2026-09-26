import { afterEach, beforeEach, expect, test, vi } from "bun:test";
import { newProject, type Project } from "../model";
import { createProjectAutosave, SAVED_LABEL } from "./projectAutosave";

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());
async function settle() {
  for (let i = 0; i < 12; i++) await Promise.resolve();
}
async function tick(ms: number) {
  vi.advanceTimersByTime(ms);
  await settle();
}
function deferred() {
  let resolve!: () => void;
  let reject!: (error: Error) => void;
  const promise = new Promise<void>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

test("opening does not write; a burst of edits saves only the latest after 1.5 seconds idle", async () => {
  const p = newProject("debounce");
  const writes: Project[] = [];
  const states: string[] = [];
  const save = createProjectAutosave(p, async (value) => {
    writes.push(value);
  });
  save.subscribe(() => states.push(save.getStatus()));
  await tick(5000);
  expect(writes).toHaveLength(0);
  save.update({ ...p, brief: "first" });
  await tick(1200);
  const latest = { ...p, brief: "latest" };
  save.update(latest);
  await tick(1499);
  expect(writes).toHaveLength(0);
  await tick(1);
  expect(writes).toEqual([latest]);
  expect(states).toEqual(["待保存", SAVED_LABEL]);
  expect(save.isDirty()).toBe(false);
  await save.flush();
  await tick(5000);
  expect(writes).toHaveLength(1);
});

test("manual or exit flush writes now, cancels the pending timer and shares in-flight writes", async () => {
  const p = newProject("flush");
  const writes: Project[] = [];
  const gate = deferred();
  const save = createProjectAutosave(p, async (value) => {
    writes.push(value);
    await gate.promise;
  });
  const first = { ...p, brief: "first" };
  save.update(first);
  const a = save.flush();
  const b = save.flush();
  await settle();
  expect(writes).toEqual([first]);
  const latest = { ...p, brief: "edited during save" };
  save.update(latest);
  gate.resolve();
  await Promise.all([a, b]);
  expect(writes).toEqual([first, latest]);
  expect(save.getStatus()).toBe(SAVED_LABEL);
  await tick(5000);
  expect(writes).toHaveLength(2);
});

test("edits during slow autosave start a fresh debounce and older completion cannot mark them saved", async () => {
  const p = newProject("slow");
  const gate = deferred();
  const writes: Project[] = [];
  const save = createProjectAutosave(p, async (value) => {
    writes.push(value);
    if (writes.length === 1) await gate.promise;
  });
  save.update({ ...p, brief: "first" });
  await tick(1500);
  expect(save.getStatus()).toBe("待保存");
  await tick(300);
  expect(save.getStatus()).toBe("保存中…");
  const latest = { ...p, brief: "new" };
  save.update(latest);
  gate.resolve();
  await settle();
  expect(save.getStatus()).toBe("待保存");
  expect(save.isDirty()).toBe(true);
  await tick(1499);
  expect(writes).toHaveLength(1);
  await tick(1);
  expect(writes[1]).toBe(latest);
  expect(save.getStatus()).toBe(SAVED_LABEL);
});

test("an idle save waiting behind a slow write takes the newest snapshot without overlapping writes", async () => {
  const p = newProject("queued");
  const gate = deferred();
  const writes: Project[] = [];
  const save = createProjectAutosave(p, async (value) => {
    writes.push(value);
    if (writes.length === 1) await gate.promise;
  });
  save.update({ ...p, brief: "first" });
  await tick(1500);
  save.update({ ...p, brief: "superseded" });
  await tick(1500);
  const latest = { ...p, brief: "latest" };
  save.update(latest);
  gate.resolve();
  await settle();
  expect(writes).toHaveLength(1);
  await tick(1500);
  expect(writes).toHaveLength(2);
  expect(writes[1]).toBe(latest);
});

test("failed saves stay dirty and flush retries the latest data instead of reporting success", async () => {
  const p = newProject("failure");
  let attempts = 0;
  const save = createProjectAutosave(p, async () => {
    if (++attempts === 1) throw new Error("disk full");
  });
  save.update({ ...p, brief: "keep me" });
  await expect(save.flush()).rejects.toThrow("disk full");
  expect(save.isDirty()).toBe(true);
  expect(save.getStatus()).toContain("保存失败");
  await save.flush();
  expect(save.getStatus()).toBe(SAVED_LABEL);
  expect(save.isDirty()).toBe(false);
  await tick(5000);
  expect(attempts).toBe(2);
});
