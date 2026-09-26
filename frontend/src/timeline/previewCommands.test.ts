import { test, expect } from "bun:test";
import { PreviewCommands } from "./previewCommands";
import { previewSpec } from "./previewSpec";
import { newProject } from "../model";

test("scrubbing drops obsolete seeks but preserves transport ordering", async () => {
  let release!: () => void;
  const calls: string[] = [];
  const queue = new PreviewCommands(
    async ({ action, frame }) => {
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
  queue.push("seek", 1);
  queue.push("seek", 2);
  queue.push("seek", 3);
  queue.push("play");
  queue.push("seek", 4);
  queue.push("seek", 5);
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
