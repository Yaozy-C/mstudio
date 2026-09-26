import { expect, test } from "bun:test";
import { createExitGuard } from "./exitGuard";

test("repeated close requests wait for one save before exiting", async () => {
  let saved = false;
  let release!: () => void;
  let saves = 0;
  let exits = 0;
  const guard = createExitGuard(
    async () => {
      saves++;
      await new Promise<void>((resolve) => {
        release = resolve;
      });
      saved = true;
    },
    async () => {
      expect(saved).toBe(true);
      exits++;
    },
  );
  const first = guard.request();
  expect(guard.request()).toBe(first);
  await Promise.resolve();
  expect(saves).toBe(1);
  expect(exits).toBe(0);
  expect(guard.getSnapshot().busy).toBe(true);
  release();
  await first;
  expect(exits).toBe(1);
  expect(guard.getSnapshot()).toEqual({ busy: false, error: "" });
});

test("failed saves keep the app open and retry saves the latest document", async () => {
  let fail = true;
  let document = "first";
  const saved: string[] = [];
  let exits = 0;
  const guard = createExitGuard(
    async () => {
      if (fail) throw new Error("disk full");
      saved.push(document);
    },
    async () => {
      exits++;
    },
  );
  await guard.request();
  expect(exits).toBe(0);
  expect(guard.getSnapshot().error).toContain("disk full");
  document = "latest";
  fail = false;
  await guard.request();
  expect(saved).toEqual(["latest"]);
  expect(exits).toBe(1);
  expect(guard.getSnapshot().error).toBe("");
});

test("returning to editing dismisses the error without exiting", async () => {
  let attempts = 0;
  let exits = 0;
  const guard = createExitGuard(
    async () => {
      attempts++;
      throw new Error("cannot save");
    },
    async () => {
      exits++;
    },
  );
  await guard.request();
  guard.dismiss();
  expect(guard.getSnapshot()).toEqual({ busy: false, error: "" });
  expect(exits).toBe(0);
  await guard.request();
  expect(attempts).toBe(2);
  expect(guard.getSnapshot().error).toContain("cannot save");
});
