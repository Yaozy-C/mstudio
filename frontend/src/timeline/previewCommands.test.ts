import { test, expect } from "bun:test";
import { PreviewCommands } from "./previewCommands";
import { previewSpec } from "./previewSpec";
import { newProject } from "../model";

test("scrubbing drops obsolete seeks but preserves transport ordering", async () => {
  let release!: () => void;
  const calls: string[] = [];
  const queue = new PreviewCommands(
    async (command) => {
      const { action } = command;
      const frame = "frame" in command ? command.frame : undefined;
      calls.push(`${action}:${frame ?? ""}`);
      if (calls.length === 1)
        await new Promise<void>((resolve) => {
          release = resolve;
        });
    },
    (error) => {
      throw error;
    },
  );
  queue.push({ action: "seek", frame: 1 });
  queue.push({ action: "seek", frame: 2 });
  queue.push({ action: "seek", frame: 3 });
  queue.push({ action: "play" });
  queue.push({ action: "seek", frame: 4 });
  queue.push({ action: "seek", frame: 5 });
  expect(queue.busy).toBe(true);
  release();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(calls).toEqual(["seek:1", "seek:3", "play:", "seek:5"]);
  expect(queue.busy).toBe(false);
});

test("track labels do not rebuild media, mute and visibility still do", () => {
  const project = newProject("performance");
  const initial = previewSpec(project);
  project.tracks[0].name = "New name";
  project.tracks[0].sourceTrackId = "association";
  expect(previewSpec(project)).toBe(initial);
  project.tracks[0].muted = true;
  expect(previewSpec(project)).not.toBe(initial);
  const muted = previewSpec(project);
  project.tracks[0].hidden = true;
  expect(previewSpec(project)).not.toBe(muted);
});

test("pending rate changes coalesce without crossing play/pause barriers", async () => {
  let release!: () => void;
  const calls: unknown[] = [];
  const queue = new PreviewCommands(
    async (command) => {
      calls.push(command);
      if (calls.length === 1)
        await new Promise<void>((resolve) => {
          release = resolve;
        });
    },
    () => {},
  );
  queue.push({ action: "pause" });
  queue.push({ action: "rate", rate: 0.25 });
  queue.push({ action: "rate", rate: 2 });
  queue.push({ action: "play" });
  queue.push({ action: "rate", rate: 0.5 });
  release();
  await new Promise((resolve) => setTimeout(resolve, 0));
  expect(calls).toEqual([
    { action: "pause" },
    { action: "rate", rate: 2 },
    { action: "play" },
    { action: "rate", rate: 0.5 },
  ]);
});
test("disposing cancels pending work and ignores failure of an in-flight command", async () => {
  let reject!: (e: unknown) => void;
  let errors = 0;
  let sent = 0;
  const queue = new PreviewCommands(
    () => {
      sent++;
      return new Promise<void>((_, r) => {
        reject = r;
      });
    },
    () => {
      errors++;
    },
  );
  queue.push({ action: "seek", frame: 1 });
  queue.push({ action: "play" });
  queue.dispose();
  reject(Error("retired"));
  await new Promise((resolve) => setTimeout(resolve, 0));
  queue.push({ action: "pause" });
  expect(sent).toBe(1);
  expect(errors).toBe(0);
  expect(queue.busy).toBe(false);
});
