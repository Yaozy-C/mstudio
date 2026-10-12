import { expect, test } from "bun:test";
import {
  requestTask,
  runsOf,
  type GenerationCommandContext,
} from "./requestTask";
import { fixture } from "./fixtures.test-helper";

test("new calls and turns cannot duplicate a shot generation; changed prompts can create variants", () => {
  const p = fixture();
  const id = p.nodes.find((n) => n.shot)!.id;
  const ctx: GenerationCommandContext = {
    callId: "first",
    turnId: "t1",
    turn: {
      projectId: p.id,
      models: { image: "image-model", video: "" },
      instruction: "generate",
    },
  };
  const op = {
    op: "request_generation",
    id,
    mediaKind: "image",
    text: "A complete shot description",
    references: [],
  };
  const next = requestTask(p, op, ctx);
  for (const status of [
    "READY",
    "IN_PROGRESS",
    "COMPLETED",
    "FAILED",
    "CANCELLED",
  ] as const) {
    const run = runsOf(next)[0];
    next.production!.drafts![run.key] = { ...run, status };
    expect(() =>
      requestTask(next, op, { ...ctx, callId: "recovery", turnId: "t2" }),
    ).toThrow(run.key);
    expect(runsOf(next)).toHaveLength(1);
  }
  const variant = requestTask(
    next,
    { ...op, text: "Different composition" },
    { ...ctx, callId: "variant", turnId: "t2" },
  );
  expect(runsOf(variant)).toHaveLength(2);
});
