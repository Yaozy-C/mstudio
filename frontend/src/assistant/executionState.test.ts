import { expect, test } from "bun:test";
import {
  executionActive,
  executionSync,
  type ExecutionState,
} from "./executionState";
import { historyMessages } from "./historyMessages";

const state = (status: ExecutionState["status"]): ExecutionState => ({
  projectId: "p",
  turnId: "t",
  status,
  text: "Script",
  error: null,
  revision: 1,
});

test("waiting and automatic followup stay active until parent delivery completes", () => {
  expect(
    ["running", "waiting", "running", "completed"].map((status) =>
      executionActive(state(status as ExecutionState["status"])),
    ),
  ).toEqual([true, true, true, false]);
  expect(executionActive(state("cancelled"))).toBe(false);
});

test("a delayed baseline cannot undo a background wake or a terminal transition", async () => {
  let resolve!: (state: ExecutionState) => void;
  const applied: (ExecutionState | null)[] = [];
  const sync = executionSync({
    read: () =>
      new Promise((yes) => {
        resolve = yes;
      }),
    apply: (s) => applied.push(s),
    error: (e) => {
      throw e;
    },
  });
  const baseline = sync.refresh();
  sync.receive(state("running"));
  sync.receive(state("completed"));
  resolve(state("waiting"));
  await baseline;
  expect(applied.map((s) => s?.status)).toEqual(["running", "completed"]);
});

test("history restores active work without a failure or retry card", () => {
  for (const status of ["running", "waiting"]) {
    const restored = historyMessages([
      {
        id: 1,
        role: "assistant",
        content: "Working",
        payload: [],
        attribution: { status, turnId: "t" },
      },
    ]);
    expect(restored[0]?.status?.type).not.toBe("incomplete");
    expect(restored[0]?.metadata?.custom?.status).toBe(status);
  }
});

test("late waiting events cannot undo cancellation or final delivery", () => {
  const applied: string[] = [];
  const sync = executionSync({
    read: async () => null,
    apply: (s) => {
      if (s) applied.push(s.status);
    },
    error: (e) => {
      throw e;
    },
  });
  sync.receive({ ...state("cancelled"), revision: 3 });
  sync.receive({ ...state("waiting"), revision: 2 });
  expect(applied).toEqual(["cancelled"]);
});
