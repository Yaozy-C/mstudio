import { expect, test } from "bun:test";
import { streamReply, type AgentProgress } from "./streamReply";
import type { bridge } from "../bridge";

function fixture() {
  let receive!: (event: AgentProgress) => void;
  let resolve!: (text: string) => void;
  let reject!: (error: Error) => void;
  let subscribed = false;
  let closed = false;
  const request = { projectId: "p", clientTurnId: "t" };
  const response = new Promise<string>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  const invoke = (async () => {
    expect(subscribed).toBe(true);
    return response;
  }) as typeof bridge;
  const stream = streamReply(
    invoke,
    request,
    new AbortController().signal,
    async (handler) => {
      receive = handler;
      subscribed = true;
      return () => {
        closed = true;
      };
    },
  );
  return {
    stream,
    resolve: (s: string) => resolve(s),
    reject: (e: Error) => reject(e),
    closed: () => closed,
    emit: (text: string, turnId = "t") =>
      receive({
        projectId: "p",
        turnId,
        kind: "assistant/partial",
        payload: { text },
      }),
  };
}
test("streams before completion, ignores another turn and closes subscription", async () => {
  const f = fixture();
  const first = f.stream.next();
  await Bun.sleep(0);
  f.emit("foreign", "other");
  f.emit("第一段");
  expect((await first).value).toBe("第一段");
  const last = f.stream.next();
  f.resolve("第一段和结尾");
  expect((await last).value).toBe("第一段和结尾");
  expect((await f.stream.next()).done).toBe(true);
  expect(f.closed()).toBe(true);
});
test("failed request retains delivered prefix and always unsubscribes", async () => {
  const f = fixture();
  const first = f.stream.next();
  await Bun.sleep(0);
  f.emit("保留这段");
  expect((await first).value).toBe("保留这段");
  const next = f.stream.next();
  f.reject(new Error("connection lost"));
  await expect(next).rejects.toThrow("connection lost");
  expect(f.closed()).toBe(true);
});
